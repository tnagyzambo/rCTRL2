use crate::attribute::{ContainerAttributes, FieldAttributes};
use virtue::generate::Generator;
use virtue::parse::Fields;
use virtue::prelude::*;

/// All information needed to generate .to_line_protocol() function for the structure that derive is being called on.
pub(crate) struct DeriveStruct {
    pub fields: Fields,
    pub attributes: ContainerAttributes,
}

impl DeriveStruct {
    pub fn generate_to_line_protocol(self, generator: &mut Generator) -> Result<()> {
        generator
            .impl_for("ToLineProtocol")
            .generate_fn("to_line_protocol")
            .with_self_arg(FnSelfArg::RefSelf)
            .with_return_type(
                "core::result::Result<LineProtocol, influx::error::LineProtocolError>",
            )
            .body(|fn_body| {
                // All influx tags and fields will be collected in vectors so .join(","") can be called
                // at the end for easy formatting
                fn_body.push_parsed(format!("let mut tags = Vec::<String>::new();"))?;
                fn_body.push_parsed(format!("let mut fields = Vec::<String>::new();"))?;
                
                // Push influx measurement onto tag vector so .join(",") can correctly format:
                // measurement,tag_key=tag_value ...
                // OR
                // measurement ...
                fn_body.push_parsed(
                    format!(
                        "tags.push(\"{}\".to_string());",
                        self.attributes.measurement
                    )
                    .to_string(),
                )?;

                // Collect all tag or field annotations for structure that derive is being called on
                for field in &self.fields.names() {
                    let attributes = field
                        .attributes()
                        .get_attribute::<FieldAttributes>()?
                        .unwrap_or_default();

                    match attributes {
                        FieldAttributes::Tag(t) => {
                            fn_body.push_parsed(format!(
                                "tags.push(format!(\"{}={{}}\", self.{}));",
                                t.unwrap_or(field.to_string()),
                                field.to_string()
                            ))?;
                        }
                        FieldAttributes::Field(f) => {
                            fn_body.push_parsed(format!("match self.{} {{
                                Some(value) => fields.push(format!(\"{}={{}}\", value.to_field_value())),
                                None => (),
                            }}", field.to_string(),  f.unwrap_or(field.to_string()),
                            ))?;
                        }
                        _ => (),
                    }
                }
                
                // Create timestamp
                fn_body.push_parsed(format!("let timestamp = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)?.{}();", self.attributes.timestamp_precision.as_function_call()))?;

                // Format line protocol
                fn_body.push_parsed(format!(
                    "let line_protocol = format!(\"{{}} {{}} {{}}\", tags.join(\",\"), fields.join(\",\"), timestamp);"
                ))?;

                fn_body.push_parsed(format!("return Ok(line_protocol);"))?;
                Ok(())
            })?;
        Ok(())
    }
}
