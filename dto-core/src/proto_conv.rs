use darling::util::path_to_string;
use darling::{Error, FromField, FromVariant};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{DataEnum, DataStruct, DeriveInput};

use crate::attributes::{
    Direction,
    ProtoFieldInfo,
    ProtoInfo,
    ProtoVariantInfo,
};
use crate::utils::{to_pascal_case, to_snake_case};

pub(crate) fn expand_proto_conv(
    direction: Direction,
    info: ProtoInfo,
    input: DeriveInput,
) -> darling::Result<TokenStream> {
    // match if this is a struct or an enum
    match input.data {
        | syn::Data::Struct(data) => expand_struct(direction, info, data),
        | syn::Data::Enum(data) => expand_enum(direction, info, data),
        | _ => {
            Err(Error::unsupported_shape(&format!(
                "{} can only be derived for structs and enums",
                direction
            )))
        }
    }
}

fn expand_struct(
    direction: Direction,
    info: ProtoInfo,
    struct_data: DataStruct,
) -> Result<TokenStream, Error> {
    // error accumulator
    let mut acc = darling::Error::accumulator();
    let mut field_tokens = Vec::with_capacity(struct_data.fields.len());
    for field in struct_data.fields {
        let Some(field_info) = acc.handle(ProtoFieldInfo::from_field(&field)) else {
            continue;
        };
        let field_tok = acc.handle(field_info.gen_tokens(direction));
        if let Some(field_tok) = field_tok {
            field_tokens.push(field_tok);
        }
    }

    let (from_type, for_type) = match direction {
        | Direction::FromProto => {
            (info.target.to_token_stream(), info.ident.to_token_stream())
        }
        | Direction::IntoProto => {
            (info.ident.to_token_stream(), info.target.to_token_stream())
        }
    };

    let tokens = quote! {
    #[automatically_derived]
    #[allow(clippy::all)]
    impl ::std::convert::From<#from_type> for #for_type {
                fn from(value: #from_type) -> Self {
                    Self {
                        #(#field_tokens)*
                    }

                }
        }
    };
    acc.finish_with(tokens)
}

fn expand_enum(
    direction: Direction,
    info: ProtoInfo,
    enum_data: DataEnum,
) -> Result<TokenStream, Error> {
    // We cheat by looking at the first variant to determine whether this is a
    // unit-only enum or not. Ideally, we should fail if there is a mix of unit
    // and unnamed.
    if let syn::Fields::Unit = enum_data.variants.first().unwrap().fields {
        expand_unit_only_enum(direction, info, enum_data)
    } else {
        expand_non_unit_enum(direction, info, enum_data)
    }
}

fn expand_non_unit_enum(
    direction: Direction,
    info: ProtoInfo,
    enum_data: DataEnum,
) -> Result<TokenStream, Error> {
    // error accumulator
    let mut acc = darling::Error::accumulator();
    // * Non unit enums *
    // let's figure out the oneof name if it wasn't set in attributes.
    // Prost has a funny way to encode a message with oneof. let's assume a
    // message like this:
    //
    // ```
    // message Foo { // --> The module name created but in snake_case.
    //   oneof bar { // --> The type of the created enum but in PascalCase.
    //     Buff baz = 1;
    //     Stuff maz = 2;
    //  }
    // }
    // ```
    //
    //
    // `some_prefix::Foo` is our target (`target=<>`), however, prost will
    // create a module `foo` with `Bar` as enum holding the variants. our
    // variants will be
    //
    // ```
    // some_prefix::foo::Bar::Baz
    // some_prefix::foo::Bar::Maz
    // ```
    //
    // In our case, we assume that our enum variants already match the target
    // variant ident (unless `name=` is passed in `proto` attribute.
    // However, we need to craft the path from the input target_type
    // (`Foo`). We assume `oneof`
    let mut target_path_segments = info.target.segments.clone();

    // `Foo` (target_path_segments is now `some_prefix::`)
    let target_path_tail_type = target_path_segments.pop().unwrap();
    // `foo`
    let target_tail_type_snakecase = Ident::new(
        &to_snake_case(&target_path_tail_type.value().ident.to_string()),
        target_path_tail_type.span(),
    );

    // `bar` (or `foo` by default)
    let oneof_ident = info
        .oneof
        .unwrap_or_else(|| target_tail_type_snakecase.clone());
    // `Bar`
    let oneof_ident_pascal =
        format_ident!("{}", to_pascal_case(&oneof_ident.to_string()));

    // `some_prefix::foo::Bar`
    let span = info.target.span();
    let fully_qualified_oneof_type = quote_spanned! { span =>
        #target_path_segments #target_tail_type_snakecase::#oneof_ident_pascal
    };

    let (source_type, target_type) = match direction {
        | Direction::FromProto => (fully_qualified_oneof_type, quote! {Self}),
        | Direction::IntoProto => {
            (info.ident.to_token_stream(), fully_qualified_oneof_type)
        }
    };

    let mut variant_tokens: Vec<_> =
        Vec::with_capacity(enum_data.variants.len());

    for variant in enum_data.variants {
        let Some(variant_info) = acc.handle(ProtoVariantInfo::from_variant(&variant)) else {
            continue;
        };
        let variant_tok = acc.handle(variant_info.gen_tokens(
            direction,
            &source_type,
            &target_type,
        ));
        if let Some(variant_tok) = variant_tok {
            variant_tokens.push(variant_tok);
        }
    }

    if *info.non_exhaustive {
        let span = info.non_exhaustive.span();
        let target_type_str = path_to_string(&info.target);
        variant_tokens.push(quote_spanned! { span =>
            _e => {
                // Missing variants.
                panic!("Attempting to convert variant of `{:?}` with no match in the target type `{}`",
                       _e, #target_type_str);
            }
        });
    }

    let (from_type, for_type) = match direction {
        | Direction::FromProto => {
            (info.target.to_token_stream(), info.ident.to_token_stream())
        }
        | Direction::IntoProto => {
            (info.ident.to_token_stream(), info.target.to_token_stream())
        }
    };

    let body = match direction {
        | Direction::IntoProto => {
            quote! {
                let o = match value {
                    #(#variant_tokens)*
                };
                Self {
                    #oneof_ident: ::std::option::Option::Some(o),
                }
            }
        }
        | Direction::FromProto => {
            quote! {
                match value.#oneof_ident.unwrap() {
                    #(#variant_tokens)*
                }
            }
        }
    };
    let tokens = quote! {
        #[automatically_derived]
        #[allow(clippy::all)]
        #[allow(unreachable_patterns)]
        impl ::std::convert::From<#from_type> for #for_type {
            fn from(value: #from_type) -> Self {
                #body
            }
        }
    };

    acc.finish_with(tokens)
}

fn expand_unit_only_enum(
    direction: Direction,
    info: ProtoInfo,
    enum_data: DataEnum,
) -> Result<TokenStream, Error> {
    // error accumulator
    let mut acc = darling::Error::accumulator();
    let mut variant_tokens: Vec<_> =
        Vec::with_capacity(enum_data.variants.len());

    let (from_type, for_type) = match direction {
        | Direction::FromProto => {
            (info.target.to_token_stream(), info.ident.to_token_stream())
        }
        | Direction::IntoProto => {
            (info.ident.to_token_stream(), info.target.to_token_stream())
        }
    };

    for variant in enum_data.variants {
        let Some(variant_info) = acc.handle(ProtoVariantInfo::from_variant(&variant)) else {
            continue;
        };
        let variant_tok = acc
            .handle(variant_info.gen_tokens(direction, &from_type, &for_type));
        if let Some(variant_tok) = variant_tok {
            variant_tokens.push(variant_tok);
        }
    }

    let target_type_str = path_to_string(&info.target);
    if *info.non_exhaustive {
        let span = info.non_exhaustive.span();
        variant_tokens.push(quote_spanned! { span =>
            _e => {
                // Missing variants.
                panic!("Attempting to convert variant of `{:?}` with no match in the target type `{}`",
                       _e, #target_type_str);
            }
        });
    }

    let tokens = match direction {
        | Direction::IntoProto => {
            quote! {
                #[automatically_derived]
                #[allow(clippy::all)]
                #[allow(unreachable_patterns)]
                impl ::std::convert::From<#from_type> for #for_type {
                    fn from(value: #from_type) -> Self {
                        match value {
                            #(#variant_tokens)*
                        }
                    }
                }

                // Provides a conversion from our enum to i32's repr of the target proto
                #[automatically_derived]
                #[allow(clippy::all)]
                #[allow(unreachable_patterns)]
                impl ::std::convert::From<#from_type> for i32 {
                    fn from(value: #from_type) -> Self {
                       let proto: #for_type = ::std::convert::Into::into(value);
                       proto as i32
                    }
                }
            }
        }
        | Direction::FromProto => {
            quote! {

            #[automatically_derived]
            #[allow(clippy::all)]
            #[allow(unreachable_patterns)]
            impl ::std::convert::From<i32> for #for_type {
                fn from(value: i32) -> Self {
                    let enum_value = #from_type::from_i32(value).unwrap();
                    match enum_value {
                        #from_type::Unknown => {
                            panic!("We should never see {}", concat!(#target_type_str, "::Unknown"));
                        },
                        #(#variant_tokens)*
                    }
                }
            }
            }
        }
    };
    acc.finish_with(tokens)
}
