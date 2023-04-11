use anyhow::Result;
use rctrl_api::remote::{Cmd, CmdEnum, Data};
use tokio::sync::mpsc;
use tracing::{event, Level};

pub struct Context {
    cmd_rx: mpsc::Receiver<Cmd>,
    data_tx: mpsc::Sender<Data>,
}

impl Context {
    pub fn new(cmd_rx: mpsc::Receiver<Cmd>, data_tx: mpsc::Sender<Data>) -> Result<Self> {
        let ctx = Self {
            cmd_rx: cmd_rx,
            data_tx: data_tx,
        };

        Ok(ctx)
    }

    pub fn run(&mut self) {
        let mut data = Data::default();

        // Recieve data from tokio runtime in a non-blocking way
        match self.cmd_rx.try_recv() {
            Ok(cmd) => match cmd.cmd {
                CmdEnum::ValveOpen => {
                    data.valve = Some(true);
                    data.log_msg = Some("valve opened".to_string());
                }
                CmdEnum::ValveClose => {
                    data.valve = Some(false);
                    data.log_msg = Some("valve closed".to_string());
                }
            },
            _ => (),
        }

        // Send data to tokio runtime in a non-blocking way
        match self.data_tx.try_send(data.clone()) {
            Err(e) => {
                event!(Level::ERROR, "failed to send data to tokio runtime: {}", e);
            }
            _ => (),
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
