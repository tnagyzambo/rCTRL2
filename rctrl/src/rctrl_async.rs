use anyhow::Result;
use bincode;
use futures_util::{SinkExt, StreamExt};
use influx::ToLineProtocolEntries;
use rctrl_api::remote::{Cmd, Data};
use std::fmt::Debug;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{event, Level};

/// Main tokio runtime loop. All task that are not safe for realtime performance should be run from this runtime.
pub async fn tokio_main(
    data_rx: mpsc::Receiver<Data>,
    cmd_tx: mpsc::Sender<Cmd>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    // Read in config
    let addr = "127.0.0.1:9090".to_string();

    // TCP socket listener to accept connections on, event loop runs in tokio executor
    let listener = TcpListener::bind(&addr).await?;
    event!(Level::INFO, "gui connection available on: {}", addr);

    let (data_latest_tx, data_latest_rx) = watch::channel(Data::default());

    let t1 = tokio::task::spawn(await_connection(listener, data_latest_rx, cmd_tx));
    let t2 = tokio::task::spawn(process_data(data_rx, data_latest_tx));

    let tasks = [t1, t2];
    tokio::select! {
       // Gui WebSocket connection handling and data logging are long running async tasks
       // We join their futures to allow for concurrent execution on the current tokio task
       // join! only returns when all futures are complete
       // If there is a fatal error on one of the tasks, the remaining will run until completion
       // These tasks should not return a value, they should be resoponsible for their own error handling
       _ = futures_util::future::join_all(tasks) => (),
       _ = shutdown_rx.changed() => (),
    };

    Ok(())
}

/// Wait for new TCP connection attempt. This task should only return if a critical error is encountered
/// by the TcpListener that would require reinitialization of the Tcp socket.
async fn await_connection(
    listener: TcpListener,
    data_latest_rx: watch::Receiver<Data>,
    cmd_tx: mpsc::Sender<Cmd>,
) {
    // Accept incoming TCP connections
    while let Ok((stream, _)) = listener.accept().await {
        let cmd_tx_c = cmd_tx.clone();
        let data_latest_rx_c = data_latest_rx.clone();

        // Join handle created by tokio::spawn is discarded
        // Created gui connections are running in a detached state
        tokio::spawn(async move {
            match accept_connection(stream, cmd_tx_c, data_latest_rx_c).await {
                Ok(addr) => event!(Level::INFO, "gui connection closed: {}", addr),
                Err(e) => event!(Level::ERROR, "gui connection fatal error: {}", e),
            }
        });
    }
}

/// Accept incoming TCP connection and attempt to promote to a WebSocket connection.
async fn accept_connection(
    stream: TcpStream,
    cmd_tx: mpsc::Sender<Cmd>,
    data_latest_rx: watch::Receiver<Data>,
) -> Result<std::net::SocketAddr> {
    // Get address of peer
    let addr = stream.peer_addr()?;

    // Promote TCP connection to WebSocket
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    event!(Level::INFO, "gui connection opened: {}", addr);

    // Split the WebSocket into Sender/Receiver halves
    // The types of ws_tx and ws_rx are a bit complicated, see ws_read() and ws_write() for details
    let (ws_tx, ws_rx) = ws_stream.split();

    // Run async read/write functions simultaneously on the current tokio task
    // select! exits on the first returned future
    // Assign and unwrap with ? returned future to allow for early exit on error
    tokio::select! {
        r = ws_read(ws_rx, cmd_tx) => r?,
        r = ws_write(ws_tx, data_latest_rx) => r?,
    };

    Ok(addr)
}

/// Process incomming data from WebSocket.
/// This function should only return on WebSocket close or fatal errors.
///
/// Some advanced trait manipulation going on here. This function is generic on Streams
/// via the TryStreamExt trait. Unlike SinkExt, the underlying data type of the Stream is not availlable
/// as a generic argument for the trait. Instead the associated type Item must be constrained to our
/// WebSocket read return type via the <Item = ...> argument provided to the StreamExt trait.
/// Additionally, the Stream must also implement Unpin (due to how streams work).
async fn ws_read<
    R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
>(
    mut ws_rx: R,
    cmd_tx: mpsc::Sender<Cmd>,
) -> Result<()> {
    while let Some(msg) = ws_rx.next().await {
        let msg = msg?;

        if msg.is_binary() {
            match bincode::deserialize::<Cmd>(&msg.into_data()) {
                Ok(cmd) => cmd_tx.send(cmd).await?,
                Err(e) => event!(
                    Level::ERROR,
                    "error deserializing incomming websocket message: {}",
                    e
                ),
            };
        }
    }

    Ok(())
}

/// Watch for changes on data_latest_rx and write them to the WebSocket.
/// This function should only return on fatal errors.
///
/// This function is generic on Sinks via the SinkExt trait. The underlying data type
/// of the stream must be provided as a generic argument to the trait as `SinkExt<Item>`.
/// Additionally, the Sink must also implement Unpin (due to how streams work)
/// and Debug (to allow ? opperator).
/// Some additional contstaints must be placed on T when it produces an error, in order for the
/// error to be thread safe.
async fn ws_write<'a, T: SinkExt<Message> + Unpin + Debug>(
    mut ws_tx: T,
    mut data_latest_rx: watch::Receiver<Data>,
) -> Result<()>
where
    <T as futures_util::Sink<Message>>::Error:
        'static + std::error::Error + std::marker::Send + Sync,
{
    while let Ok(()) = data_latest_rx.changed().await {
        // I don't like that this data needs to be cloned twice
        let data = data_latest_rx.borrow().clone();

        match bincode::serialize(&data) {
            Ok(msg) => ws_tx.send(Message::Binary(msg)).await?,
            Err(e) => event!(
                Level::ERROR,
                "failed to serialize outgoing websocket meesage: {}",
                e
            ),
        }
    }

    Ok(())
}

/// Log all data recieved on the data_rx mspc channel to InfluxDB.
/// Retransmit recieved data at a reduced rate to the WebSocket.
/// Some performance considerations for this fucntion: constant reallocation of the influx write buffer
/// is unwanted. A single pre-allocation is made for every batch write to InfluxDB. If the buffer fills up
/// and has to reallocate, the new larger size is used for the next pre-allocation.
///
/// Ideally, a shared memory pool is created once, and portions of the memory pool are used and freed as
/// they are needed by the spawned tokio tasks. This is complicated, and not currently implemented.
async fn process_data(mut data_rx: mpsc::Receiver<Data>, data_latest_tx: watch::Sender<Data>) {
    let mut last_data_latest_tx = std::time::Instant::now();
    let mut influx_write_buf_capacity = 20;

    loop {
        // Pre-allocate buffer string
        let mut influx_write_buf = String::with_capacity(influx_write_buf_capacity);
        let mut influx_write_entries = 0;

        while let Some(data) = data_rx.recv().await {
            // Every 15ms update the WebSocket
            // If the WebSocket crashes the send will fail, there is nothing that we can do about it
            // so we ignore the error
            if last_data_latest_tx.elapsed().as_millis() > 15 {
                _ = data_latest_tx.send(data.clone());
                last_data_latest_tx = std::time::Instant::now();
            }

            // Convert data to line protocol and write to buffer
            match data.to_line_protocol_entries() {
                Ok(mut line_protocol_entries) => {
                    while let Some(line_protocol_entry) = line_protocol_entries.pop() {
                        influx_write_buf.push_str(line_protocol_entry.as_str());
                        influx_write_entries += 1;
                    }
                }
                Err(e) => event!(
                    Level::ERROR,
                    "failed to convert data to line protocol entries: {:?}",
                    e
                ),
            }

            // Write to influx in ~5000 line batches
            if influx_write_entries > 50 {
                if influx_write_buf.len() > influx_write_buf_capacity {
                    influx_write_buf_capacity = influx_write_buf.len();
                    event!(
                        Level::INFO,
                        "grew capaicty of influx write buffer to {}",
                        influx_write_buf_capacity
                    );
                }

                tokio::task::spawn(write_to_influx(influx_write_buf));
                break;
            }
        }
    }
}

async fn write_to_influx(data: String) {}
