use darling::Error;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, DeriveInput};

use crate::attributes::ProstMessageInto;

pub(crate) fn expand_prost_ext(
    info: ProstMessageInto,
    input: DeriveInput,
) -> darling::Result<TokenStream> {
    match input.data {
        | syn::Data::Struct(data) => expand_struct(info, data),
        | _ => {
            Err(Error::unsupported_shape(
                "can only be derived for prost structs",
            ))
        }
    }
}

fn expand_struct(
    info: ProstMessageInto,
    _struct_data: DataStruct,
) -> Result<TokenStream, Error> {
    // For future use:
    // for field in struct_data.fields {
    //     let field_info = ProstFieldInfo::from_field(&field)?;
    // }
    let struct_type = &info.ident;

    Ok(quote! {
        impl ::dto::traits::ProstLazyDefault for #struct_type {
            fn default_instance() -> &'static Self {
                // Type name must be explicit in static.
                static INSTANCE: ::dto::exports::once_cell::sync::Lazy<#struct_type> = ::dto::exports::once_cell::sync::Lazy::new(::std::default::Default::default);
                &INSTANCE
            }
        }
    })
}
