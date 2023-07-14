use darling::Error;
use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::proto_attrs::FromProto;

pub(crate) fn expand_from_proto(
    _info: FromProto,
    _input: DeriveInput,
) -> Result<TokenStream, Error> {
    // match if this is a struct or an enum
    todo!()
}
