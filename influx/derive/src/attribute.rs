use virtue::prelude::*;
use virtue::utils::{parse_tagged_attribute, ParsedAttribute};

pub enum TimestampPrecision {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
}

impl TimestampPrecision {
    pub fn as_function_call(&self) -> String {
        match *self {
            TimestampPrecision::Nanoseconds => "as_nanos".to_string(),
            TimestampPrecision::Microseconds => "as_micros".to_string(),
            TimestampPrecision::Milliseconds => "as_millis".to_string(),
            TimestampPrecision::Seconds => "as_secs".to_string(),
        }
    }
}

pub struct ContainerAttributes {
    pub measurement: String,
    pub timestamp_precision: TimestampPrecision,
}

impl Default for ContainerAttributes {
    fn default() -> Self {
        Self {
            measurement: "measurment".to_string(),
            timestamp_precision: TimestampPrecision::Nanoseconds,
        }
    }
}

impl FromAttribute for ContainerAttributes {
    fn parse(group: &Group) -> Result<Option<Self>> {
        let attributes = match parse_tagged_attribute(group, "influx")? {
            Some(body) => body,
            None => return Ok(None),
        };

        let mut result = Self::default();

        for attribute in attributes {
            match attribute {
                ParsedAttribute::Property(key, value) if key.to_string() == "measurement" => {
                    let value_string = value.to_string();
                    if value_string.starts_with('"') && value_string.ends_with('"') {
                        result.measurement = value_string[1..value_string.len() - 1].to_string();
                    } else {
                        return Err(Error::custom_at("should be a literal str", value.span()));
                    }
                }
                ParsedAttribute::Property(key, value)
                    if key.to_string() == "timestamp_precision" =>
                {
                    let value_string = value.to_string();
                    if value_string.starts_with('"') && value_string.ends_with('"') {
                        let precision_string = value_string[1..value_string.len() - 1].to_string();

                        match precision_string.as_str() {
                            "nanoseconds" => {
                                result.timestamp_precision = TimestampPrecision::Nanoseconds
                            }
                            "microseconds" => {
                                result.timestamp_precision = TimestampPrecision::Microseconds
                            }
                            "milliseconds" => {
                                result.timestamp_precision = TimestampPrecision::Milliseconds
                            }
                            "seconds" => result.timestamp_precision = TimestampPrecision::Seconds,
                            _ => {
                                return Err(Error::custom_at(
                                    "unknown timestamp precision",
                                    value.span(),
                                ))
                            }
                        }
                    } else {
                        return Err(Error::custom_at("should be a literal str", value.span()));
                    }
                }
                ParsedAttribute::Tag(i) => {
                    return Err(Error::custom_at("unknown container attribute", i.span()))
                }
                ParsedAttribute::Property(key, _) => {
                    return Err(Error::custom_at("unknown container attribute", key.span()))
                }
                _ => {}
            }
        }

        Ok(Some(result))
    }
}

pub enum FieldAttributes {
    Tag(Option<String>),
    Field(Option<String>),
    Untracked,
}

impl Default for FieldAttributes {
    fn default() -> Self {
        FieldAttributes::Untracked
    }
}

impl FromAttribute for FieldAttributes {
    fn parse(group: &Group) -> Result<Option<Self>> {
        let attributes = match parse_tagged_attribute(group, "influx")? {
            Some(body) => body,
            None => return Ok(None),
        };

        let mut result = Self::default();

        for attribute in attributes {
            match attribute {
                ParsedAttribute::Property(key, value) if key.to_string() == "tag" => {
                    let value_string = value.to_string();
                    if value_string.starts_with('"') && value_string.ends_with('"') {
                        result = FieldAttributes::Tag(Some(
                            value_string[1..value_string.len() - 1].to_string(),
                        ));
                    }
                }
                ParsedAttribute::Tag(i) if i.to_string() == "tag" => {
                    result = FieldAttributes::Tag(None);
                }
                ParsedAttribute::Property(key, value) if key.to_string() == "field" => {
                    let value_string = value.to_string();
                    if value_string.starts_with('"') && value_string.ends_with('"') {
                        result = FieldAttributes::Field(Some(
                            value_string[1..value_string.len() - 1].to_string(),
                        ));
                    }
                }
                ParsedAttribute::Tag(i) if i.to_string() == "field" => {
                    result = FieldAttributes::Field(None);
                }
                ParsedAttribute::Tag(i) => {
                    return Err(Error::custom_at("Unknown field attribute", i.span()))
                }
                ParsedAttribute::Property(key, _) => {
                    return Err(Error::custom_at("Unknown field attribute", key.span()))
                }
                _ => {}
            }
        }

        Ok(Some(result))
    }
}
