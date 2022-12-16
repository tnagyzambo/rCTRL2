pub use influx_derive::*;

pub mod error;

pub type LineProtocol = String;

pub trait ToLineProtocol {
    fn to_line_protocol(&self) -> Result<LineProtocol, error::LineProtocolError>;
}

pub trait ToFieldValue {
    fn to_field_value(&self) -> String;
}

impl ToFieldValue for f64 {
    fn to_field_value(&self) -> String {
        self.to_string()
    }
}

impl ToFieldValue for i64 {
    fn to_field_value(&self) -> String {
        format!("{}i", self.to_string())
    }
}

impl ToFieldValue for u64 {
    fn to_field_value(&self) -> String {
        format!("{}u", self.to_string())
    }
}

//TODO implement string to influx field
//impl ToFieldValue for String {
//    fn to_field_value(&self) -> String {
//    }
//}

impl ToFieldValue for bool {
    fn to_field_value(&self) -> String {
        format!("{}", self.to_string())
    }
}
