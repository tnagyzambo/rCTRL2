//! Influx is a crate for writing to InfluxDB. A derive macro is provided to generate a valid [line protocol][lp]
//! entry. To use annotate your data structure with the following options:
//!
//! ```
//! #[derive(ToLineProtocol)]
//! #[influx(measuremment = "influx_measurement")] // Optional overide of line protcol measurement (default is "measurment")
//! #[influx(timestamp_precision = "milliseconds")] // Optional overide of timestamp precision (default is "nanoseconds")
//! struct Data {
//!     untracked_data: f64, // Struct memebers with no annotations will not appear in the resulting line protocol
//!     
//!     #[influx(tag)]
//!     location: String, // Resulting tag set is "location=self.location.to_string()"
//!
//!     #[influx(tag = "id")]
//!     magic_number: i64, // Resulting tag set is "id=self.magic_number.to_string()"
//!
//!     #[influx(field)]
//!     value: f64 // Resulting field set is "value=self.value.to_field_value()"
//!
//!    #[influx(field = "value2")]
//!    second_value: f64 // Resulting field set is "value2=self.second_value.to_field_value()"
//! }
//! ```
//!
//! [lp]: https://docs.influxdata.com/influxdb/v2.6/reference/syntax/line-protocol/

pub use influx_derive::{ToLineProtocol, ToLineProtocolEntries};

pub mod error;

/// Valid line protocol.
pub type LineProtocol = String;

/// To valid influx line protocol
pub trait ToLineProtocol {
    fn to_line_protocol(&self) -> Result<LineProtocol, error::LineProtocolError>;
}

pub trait ToLineProtocolEntries {
    fn to_line_protocol_entries(&self) -> Result<Vec<LineProtocol>, error::LineProtocolError>;
}

/// To valid influx field value.
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

// TODO: Implement string to influx field
//impl ToFieldValue for String {
//    fn to_field_value(&self) -> String {
//    }
//}

impl ToFieldValue for bool {
    fn to_field_value(&self) -> String {
        format!("{}", self.to_string())
    }
}
