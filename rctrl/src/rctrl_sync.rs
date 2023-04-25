use anyhow::Result;
use rctrl_api::remote::{Cmd, CmdEnum, Data};
use rctrl_hw::adc::ads101x;
use rctrl_hw::adc::ads101x::ADS101x;
use rctrl_hw::sensor::KellerPA7LC;
use tokio::sync::mpsc;
use tracing::{event, Level};

// Context to contain all data needed to run syncronous logic
pub struct Context {
    cmd_rx: mpsc::Receiver<Cmd>,
    data_tx: mpsc::Sender<Data>,

    adc: ADC,
    sensor: Sensor,
}

impl Context {
    // Perform all sensor and IO initializations here
    pub fn new(cmd_rx: mpsc::Receiver<Cmd>, data_tx: mpsc::Sender<Data>) -> Result<Self> {
        let ctx = Self {
            cmd_rx,
            data_tx,
            adc: ADC::new()?,
            sensor: Sensor::new()?,
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

        data.sensor = match self.adc.fc_ads1014_no1.read(&self.sensor.pressure) {
            Ok(pressure) => Some(pressure),
            Err(e) => {
                // TODO: improve error handling/clarity of error
                event!(Level::ERROR, "failed to read sensor: {}", e);
                None
            }
        };
    }
}

struct ADC {
    fc_ads1014_no1: ADS101x,
}

impl ADC {
    fn new() -> Result<Self> {
        let mut fc_ads1014_no1 = ADS101x::new("path", 0x00)?;
        fc_ads1014_no1.config(
            ads101x::Config::default()
                .with_os(ads101x::Os::On)
                .with_mux(ads101x::Mux::Ain0Ain3),
        )?;
        Ok(Self { fc_ads1014_no1 })
    }
}

struct Sensor {
    pressure: KellerPA7LC,
}

impl Sensor {
    fn new() -> Result<Self> {
        Ok(Self {
            pressure: KellerPA7LC::new(),
        })
    }
}
