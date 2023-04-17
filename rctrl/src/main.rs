use ctrlc;
use rctrl_api::remote::{Cmd, Data};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::runtime::Builder;
use tokio::sync::{mpsc, watch};
use tracing::{event, Level};
use tracing_subscriber;

mod rctrl_async;
mod rctrl_sync;

fn main() {
    tracing_subscriber::fmt::init();

    let (data_tx, data_rx) = mpsc::channel::<Data>(16);
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(16);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create new single threaded runtime
    let rt = Builder::new_current_thread().enable_all().build().unwrap();

    // Run tokio runtime on new thread
    // Fatal errors on the tokio runtime thread should not crash the main sync thread
    // Joining a thread handle moves it out to prevent a thread from being closed twice
    // as a result the join handle does not implement clone() and needs to be wrapped in
    // an Option<()> in order to be shared
    let mut tokio_handle = Some(std::thread::spawn(move || {
        rt.block_on(async move {
            match rctrl_async::tokio_main(data_rx, cmd_tx, shutdown_rx).await {
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
        running_c.store(false, Ordering::SeqCst);

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
    }) {
        Ok(()) => (),
        Err(e) => {
            event!(Level::ERROR, "failed to set ctrlc handler: {}", e);
            return;
        }
    }

    // Create syncronous logic context
    // This invloves steps such as hardware initialization so might fail
    // Failure to create the syncronous logic context should result in a fatal error
    let mut sync_ctx = match rctrl_sync::Context::new(cmd_rx, data_tx) {
        Ok(ctx) => ctx,
        Err(e) => {
            event!(Level::ERROR, "failed to create sync context: {}", e);
            return;
        }
    };

    // Run syncronous logic
    while running.load(Ordering::SeqCst) {
        sync_ctx.run()
    }

    event!(Level::INFO, "exited");
}
