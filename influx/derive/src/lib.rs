mod attribute;
mod derive_struct;

use attribute::ContainerAttributes;
use virtue::prelude::*;

#[proc_macro_derive(ToLineProtocol, attributes(influx))]
pub fn derive_to_line_protocol(input: TokenStream) -> TokenStream {
    derive_to_line_protocol_inner(input).unwrap_or_else(|e| e.into_token_stream())
}

fn derive_to_line_protocol_inner(input: TokenStream) -> Result<TokenStream> {
    let parse = Parse::new(input)?;
    let (mut generator, attributes, body) = parse.into_generator();
    let attributes = attributes
        .get_attribute::<ContainerAttributes>()?
        .unwrap_or_default();

    match body {
        Body::Struct(body) => {
            derive_struct::DeriveStruct {
                fields: body.fields,
                attributes,
            }
            .generate_to_line_protocol(&mut generator)?;
        }
        Body::Enum(body) => {
            //TODO: impletement enum encoding
        }
    }

    generator.finish()
}
