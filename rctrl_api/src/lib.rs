pub mod remote {
    use crate::sensor::Pressure;
    use influx::{LineProtocol, ToLineProtocol, ToLineProtocolEntries};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize, Default, Debug, ToLineProtocolEntries)]
    pub struct Data {
        pub sensor: Option<Pressure>,
        #[influx(untracked)]
        pub valve: Option<bool>,
        #[influx(untracked)]
        pub log_msg: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Cmd {
        pub cmd: CmdEnum,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum CmdEnum {
        ValveOpen,
        ValveClose,
    }
}

pub mod sensor {
    use influx::{LineProtocol, ToFieldValue, ToLineProtocol};
    use serde::{Deserialize, Serialize};
    use strum::Display;

    #[derive(Clone, Copy, Debug, Deserialize, Display, Serialize)]
    pub enum PressureUnit {
        Bar,
    }

    #[derive(Clone, Copy, Debug, Deserialize, Serialize, ToLineProtocol)]
    #[influx(measurement = "pressure")]
    pub struct Pressure {
        #[influx(field = "pressure")]
        pub pressure: f64,
        #[influx(tag)]
        pub unit: PressureUnit,
    }
}
