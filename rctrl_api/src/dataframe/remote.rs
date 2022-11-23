use ewebsock::WsMessage;
use std::convert::From;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct DataFrameRemote {}

impl From<WsMessage> for DataFrameRemote {
    fn from(msg: WsMessage) -> Self {
        Self {}
    }
}
