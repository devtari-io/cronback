use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use syn::DeriveInput;
mod attributes;
mod enum_codegen;
mod proto_conv;
mod struct_codegen;
mod transformers;
mod utils;

use self::attributes::{Direction, ProtoInfo};

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
