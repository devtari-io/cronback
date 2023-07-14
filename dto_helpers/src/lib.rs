mod from_proto;
mod into_proto;
mod proto_attrs;
mod utils;

use darling::FromDeriveInput;
use into_proto::expand_into_proto;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

use self::from_proto::expand_from_proto;
use self::proto_attrs::{FromProto, IntoProto};

#[proc_macro_derive(IntoProto, attributes(proto, into_proto))]
pub fn derive_into_proto(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let tokens = IntoProto::from_derive_input(&input)
        .and_then(|info| expand_into_proto(info, input))
        .map(Into::into);

    tokens.unwrap_or_else(|e| TokenStream::from(e.write_errors()))
}

#[proc_macro_derive(FromProto, attributes(proto, from_proto))]
pub fn derive_from_proto(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let tokens = FromProto::from_derive_input(&input)
        .and_then(|info| expand_from_proto(info, input))
        .map(Into::into);

    tokens.unwrap_or_else(|e| TokenStream::from(e.write_errors()))
}
