pub mod remote {
    use crate::sensor::Pressure;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize, Default, Debug)]
    pub struct Data {
        pub sensor: Option<Pressure>,
        pub valve: Option<bool>,
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
    //    use influx::{LineProtocol, ToFieldValue, ToLineProtocol};
    use serde::{Deserialize, Serialize};
    use strum::Display;
    //
    #[derive(Clone, Debug, Display, Serialize, Deserialize)]
    pub enum PressureUnit {
        Bar,
    }
    //
    #[derive(Clone, Serialize, Deserialize, Debug)]
    //    #[influx(measurement = "pressure")]
    pub struct Pressure {
        //        #[influx(field = "pressure")]
        pub pressure: f64,
        //        #[influx(tag)]
        pub unit: PressureUnit,
    }
}

//impl remote::Data {
//    pub fn to_influx_entries(self) -> Vec<LineProtocol> {
//        let mut line_protocol_vec = Vec::<LineProtocol>::new();

//        match self.sensor.try_into() {
//            Ok(line_protocol) => line_protocol_vec.push(line_protocol),
//            _ => (),
//        };

//        return line_protocol_vec;
//    }
//}

//type LineProtocol = String;

//impl TryInto<LineProtocol> for sensor::Pressure {
//    type Error = &'static str;

//    fn try_into(self) -> Result<LineProtocol, Self::Error> {
//        match self.pressure {
//            Some(value) => Ok(std::format!(
//                "pressure,sensor_id={},sensor_location={},units={} pressure={}",
//                self.id,
//                self.location,
//                self.units,
//                value
//            )),
//            None => Err("attempted to creat influx line protocol entry on None field"),
//        }
//    }
//}
