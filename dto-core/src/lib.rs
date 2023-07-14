use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use syn::DeriveInput;
mod attributes;
mod enum_codegen;
mod prost_ext;
mod proto_conv;
mod struct_codegen;
pub mod traits;
mod transformers;
mod utils;

use self::attributes::{Direction, ProstMessageInto, ProtoInfo};

pub fn derive_into_proto(input: DeriveInput) -> TokenStream {
    derive_proto(Direction::IntoProto, input)
}

pub fn derive_from_proto(input: DeriveInput) -> TokenStream {
    derive_proto(Direction::FromProto, input)
}

fn derive_proto(direction: Direction, input: DeriveInput) -> TokenStream {
    let tokens = ProtoInfo::from_derive_input(&input)
        .and_then(|info| proto_conv::expand_proto_conv(direction, info, input));

    match tokens {
        | Ok(tokens) => tokens,
        | Err(e) => e.write_errors(),
    }
}

pub fn derive_prost_message_ext(input: DeriveInput) -> TokenStream {
    let tokens = ProstMessageInto::from_derive_input(&input)
        .and_then(|info| prost_ext::expand_prost_ext(info, input));

    match tokens {
        | Ok(tokens) => tokens,
        | Err(e) => e.write_errors(),
    }
}
