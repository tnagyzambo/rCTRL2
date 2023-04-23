use super::Sensor;
use rctrl_api::sensor::{Pressure, PressureUnit};

pub struct KellerPA7LC {}

impl KellerPA7LC {
    pub fn new() -> Self {
        let sensor = Self {};

        return sensor;
    }
}

impl Sensor for KellerPA7LC {
    type Output = Pressure;

    fn conversion(&self, voltage: f64) -> Pressure {
        return Pressure {
            pressure: voltage,
            unit: PressureUnit::Bar,
        };
    }
}
