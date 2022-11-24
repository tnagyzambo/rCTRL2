pub mod remote {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Default, Debug)]
    pub struct Data {
        pub sensor: Option<f32>,
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
