use anyhow::Result;
use bincode;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{event, Instrument, Level};
use tracing_subscriber;

#[derive(Serialize, Deserialize, Default, Debug)]
struct Data {
    i: i32,
}

struct Command {}

fn main() {
    tracing_subscriber::fmt::init();

    let (data_tx, data_rx) = mpsc::channel::<Data>(16);
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(16);

    let rt = Builder::new_current_thread().enable_all().build().unwrap();

    // Run tokio runtime on new thread
    // Fatal errors on the tokio runtime thread should not crash the main sync thread
    std::thread::spawn(move || {
        rt.block_on(async move {
            match tokio_main(data_rx, cmd_tx).await {
                Ok(()) => event!(Level::INFO, "tokio runtime exited successfully"),
                Err(e) => event!(Level::ERROR, "tokio runtime exited with error: {}", e),
            }
        });
    });

    // MAIN SYNC LOGIC HERE
    let mut i: i32 = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        i = i + 1;
        data_tx.blocking_send(Data { i: i.clone() });
    }
}

/// Main tokio runtime loop. All task that are not safe for realtime performance should be run from this runtime.
async fn tokio_main(data_rx: mpsc::Receiver<Data>, cmd_tx: mpsc::Sender<Command>) -> Result<()> {
    // Read in config
    let addr = "127.0.0.1:9090".to_string();

    // TCP socket listener to accept connections on, event loop runs in tokio executor
    let listener = TcpListener::bind(&addr).await?;
    event!(Level::INFO, "gui connection avaiable on: {}", addr);

    let (data_latest_tx, data_latest_rx) = watch::channel(Data::default());

    // Gui WebSocket connection handling and data logging are long running async tasks
    // We join their futures to allow for concurrent execution on the current tokio task
    // join! only returns when all futures are complete
    // If there is a fatal error on one of the tasks, the remaining will run until completion
    // These tasks should not return a value, they should be resoponsible for their own error handling
    tokio::join!(
        await_connection(listener, data_latest_rx, cmd_tx),
        process_data(data_rx, data_latest_tx),
    );

    Ok(())
}

/// Wait for new TCP connection attempt. This task should only return if a critical error is encountered
/// by the TcpListener that would require reinitialization of the Tcp socket.
async fn await_connection(
    listener: TcpListener,
    data_latest_rx: watch::Receiver<Data>,
    cmd_tx: mpsc::Sender<Command>,
) {
    // Accept incoming TCP connections
    while let Ok((stream, _)) = listener.accept().await {
        let cmd_tx_c = cmd_tx.clone();
        let data_latest_rx_c = data_latest_rx.clone();

        // Join handle created by tokio::spawn is discarded
        // Created gui connections are running in a detached state
        tokio::spawn(async move {
            match accept_connection(stream, cmd_tx_c, data_latest_rx_c).await {
                Ok(()) => (),
                Err(e) => event!(Level::ERROR, "gui connection fatal error: {}", e),
            }
        });
    }
}

/// Accept incoming TCP connection and attempt to promote to a WebSocket connection.
async fn accept_connection(
    stream: TcpStream,
    cmd_tx: mpsc::Sender<Command>,
    data_latest_rx: watch::Receiver<Data>,
) -> Result<()> {
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
    let mut _r = Ok(());
    tokio::select! {
        r = ws_read(ws_rx, cmd_tx) => _r = r,
        r = ws_write(ws_tx, data_latest_rx) => _r = r,
    };
    _r?;

    event!(Level::INFO, "gui connection closed: {}", addr);

    Ok(())
}

/// Process incomming data from WebSocket.
/// This function should only return on WebSocket close or fatal errors.
///
/// Some advanced trait manipulation going on here. This function is generic on Streams
/// via the StreamExt trait. Unlike SinkExt, the underlying data type of the Stream is not available
/// as a generic argument for the trait. Instead the associated type Item must be constrained to our
/// WebSocket read return type via the <Item = ...> argument provided to the StreamExt trait.
/// Additionally, the Stream must also implement Unpin (due to how streams work).
async fn ws_read<
    R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
>(
    mut ws_rx: R,
    cmd_tx: mpsc::Sender<Command>,
) -> Result<()> {
    while let Some(msg) = ws_rx.next().await {
        let msg = msg?;
        match msg {
            _ => event!(Level::DEBUG, "{:?}", msg),
        }
    }

    Ok(())
}

/// Watch for changes on data_latest_rx and write them to the WebSocket.
/// This function should only return on fatal errors.
///
/// This function is generic on Sinks via the SinkExt trait. The underlying data type
/// of the stream must be provided as a generic argument to the trait as SinkExt<Item>.
/// Additionally, the Sink must also implement Unpin (due to how streams work)
/// and Debug (to allow ? opperator).
async fn ws_write<T: SinkExt<Message> + Unpin + Debug>(
    mut ws_tx: T,
    mut data_latest_rx: watch::Receiver<Data>,
) -> Result<()>
where
    <T as futures_util::Sink<Message>>::Error: Debug,
{
    while let Ok(data) = data_latest_rx.changed().await {
        ws_tx
            .send(Message::Binary(bincode::serialize(&data)?))
            .await;
    }

    Ok(())
}

async fn process_data(mut data_rx: mpsc::Receiver<Data>, data_latest_tx: watch::Sender<Data>) {
    while let Some(data) = data_rx.recv().await {
        data_latest_tx.send(data).unwrap(); // THIS CAN FAIL IF WEBSOCKET CRASHES
    }

    event!(Level::INFO, "process_exit");
}
