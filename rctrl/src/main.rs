use anyhow::Result;
use bincode;
use ctrlc;
use futures_util::{SinkExt, StreamExt};
use rctrl_api::remote::{Cmd, CmdEnum, Data};
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{event, Level};
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();

    let (data_tx, data_rx) = mpsc::channel::<Data>(16);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(16);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let rt = Builder::new_current_thread().enable_all().build().unwrap();

    // Run tokio runtime on new thread
    // Fatal errors on the tokio runtime thread should not crash the main sync thread
    // Joining a thread handle moves it out to prevent a thread from being closed twice
    // as a result the join handle does not implement clone() and needs to be wrapped in
    // an Option<()> in order to be shared
    let mut tokio_handle = Some(std::thread::spawn(move || {
        rt.block_on(async move {
            match tokio_main(data_rx, cmd_tx, shutdown_rx).await {
                Ok(()) => event!(Level::INFO, "tokio runtime exited successfully"),
                Err(e) => event!(Level::ERROR, "tokio runtime exited with error: {}", e),
            }
        });
    }));

    // Hook into ctrl + c shut down signal
    // We want to send a shutdown signal to the tokio runtime so it can clean up after itself
    // Wait for cleanup to finish and then exit the program by setting the running flag to false
    let running = Arc::new(AtomicBool::new(true));
    let running_c = running.clone();
    match ctrlc::set_handler(move || {
        event!(Level::INFO, "exiting...");
        match shutdown_tx.send(true) {
            Ok(()) => (),
            Err(e) => event!(
                Level::ERROR,
                "failed to send shutdown signal to tokio: {}",
                e
            ),
        };

        // Have to match on the thread handle existing as it might have crashed in the background
        match tokio_handle.take() {
            Some(thread) => thread.join().unwrap(),
            None => (),
        };

        running_c.store(false, Ordering::SeqCst);
    }) {
        Ok(()) => (),
        Err(e) => {
            event!(Level::ERROR, "failed to set ctrlc handler: {}", e);
            return;
        }
    }

    // MAIN SYNC LOGIC HERE
    let mut sensor: f32 = 0.0;
    let mut valve: bool = false;
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
        sensor = sensor + 0.1;

        let mut log_msg: Option<String> = None;
        match cmd_rx.try_recv() {
            Ok(cmd) => match cmd.cmd {
                CmdEnum::ValveOpen => {
                    valve = true;
                    log_msg = Some("valve opened".to_string());
                }
                CmdEnum::ValveClose => {
                    valve = false;
                    log_msg = Some("valve closed".to_string());
                }
            },
            _ => (),
        }

        data_tx
            .blocking_send(Data {
                sensor: Some(sensor.clone()),
                valve: Some(valve.clone()),
                log_msg,
            })
            .unwrap_or_else(|e| event!(Level::ERROR, "failed to send data to tokio: {}", e));
    }

    event!(Level::INFO, "exited");
}

/// Main tokio runtime loop. All task that are not safe for realtime performance should be run from this runtime.
async fn tokio_main(
    data_rx: mpsc::Receiver<Data>,
    cmd_tx: mpsc::Sender<Cmd>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    // Read in config
    let addr = "127.0.0.1:9090".to_string();

    // TCP socket listener to accept connections on, event loop runs in tokio executor
    let listener = TcpListener::bind(&addr).await?;
    event!(Level::INFO, "gui connection avaiable on: {}", addr);

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
/// via the TryStreamExt trait. Unlike SinkExt, the underlying data type of the Stream is not available
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
/// of the stream must be provided as a generic argument to the trait as SinkExt<Item>.
/// Additionally, the Sink must also implement Unpin (due to how streams work)
/// and Debug (to allow ? opperator).
/// Some additional contstaints must be placed on T when it produces an error, in order for the
/// error to be thread safe.
async fn ws_write<T: SinkExt<Message> + Unpin + Debug>(
    mut ws_tx: T,
    mut data_latest_rx: watch::Receiver<Data>,
) -> Result<()>
where
    <T as futures_util::Sink<Message>>::Error:
        'static + std::error::Error + std::marker::Send + Sync,
{
    while let Ok(data) = data_latest_rx.changed().await {
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

/// DISPATCH DATA TO WEBSOCKET AND DATALOGGER
async fn process_data(mut data_rx: mpsc::Receiver<Data>, data_latest_tx: watch::Sender<Data>) {
    while let Some(data) = data_rx.recv().await {
        data_latest_tx.send(data).unwrap(); // THIS CAN FAIL IF WEBSOCKET CRASHES
    }

    event!(Level::INFO, "process_exit");
}
