use anyhow::Result;
use rctrl_api::remote::{Cmd, CmdEnum, Data};
use rctrl_hw::adc::ads101x::ADS101x;
use rctrl_hw::sensor::KellerPA7LC;
use tokio::sync::mpsc;
use tracing::{event, Level};

// Context to contain all data needed to run syncronous logic
pub struct Context {
    cmd_rx: mpsc::Receiver<Cmd>,
    data_tx: mpsc::Sender<Data>,

    sensors: Sensors,
}

impl Context {
    // Perform all sensor and IO initializations here
    pub fn new(cmd_rx: mpsc::Receiver<Cmd>, data_tx: mpsc::Sender<Data>) -> Result<Self> {
        let ctx = Self {
            cmd_rx,
            data_tx,
            sensors: Sensors::new()?,
        };

        Ok(ctx)
    }

    // Perform all syncronous logic here
    pub fn run(&mut self) {
        let mut data = Data::default();

        // Recieve commands from tokio runtime in a non-blocking way
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

        std::thread::sleep(std::time::Duration::from_millis(500));

        match self.sensors.pressure.read() {
            Ok(value) => println!("{:?}", value),
            Err(e) => (),
        };
    }
}

struct Sensors {
    pressure: KellerPA7LC<ADS101x>,
}

impl Sensors {
    fn new() -> Result<Self> {
        let mut pressure_adc = ADS101x::new("path", 0x00)?;
        pressure_adc.config(|config| config.with_os_on().build())?;
        let pressure = KellerPA7LC::new(pressure_adc);

        Ok(Self { pressure })
    }
}
