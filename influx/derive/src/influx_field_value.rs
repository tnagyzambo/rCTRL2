pub trait ToInfluxFieldValue {
    fn to_influx_field_value(&self) -> String;
}

impl ToInfluxFieldValue for f64 {
    fn to_influx_field_value(&self) -> String {
        self.to_string()
    }
}
