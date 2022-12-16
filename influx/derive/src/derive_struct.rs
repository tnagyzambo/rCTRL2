use crate::attribute::{ContainerAttributes, FieldAttributes};
use virtue::generate::Generator;
use virtue::parse::Fields;
use virtue::prelude::*;

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
                fn_body.push_parsed(format!("let mut tags = Vec::<String>::new();"))?;
                fn_body.push_parsed(format!("let mut fields = Vec::<String>::new();"))?;
                
                fn_body.push_parsed(
                    format!(
                        "tags.push(\"{}\".to_string());",
                        self.attributes.measurement
                    )
                    .to_string(),
                )?;

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
                
                fn_body.push_parsed(format!("let timestamp = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)?.{}();", self.attributes.timestamp_precision.as_function_call()))?;

                fn_body.push_parsed(format!(
                    "let line_protocol = format!(\"{{}} {{}} {{}}\", tags.join(\",\"), fields.join(\",\"), timestamp);"
                ))?;

                fn_body.push_parsed(format!("return Ok(line_protocol);"))?;
                Ok(())
            })?;
        Ok(())
    }
}
