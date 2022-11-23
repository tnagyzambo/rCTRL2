use ewebsock::WsMessage;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct DataFrameTelemetry {}

impl From<WsMessage> for DataFrameTelemetry {
    fn from(msg: WsMessage) -> Self {
        Self {}
    }
}
