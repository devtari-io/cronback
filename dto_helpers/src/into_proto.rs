use darling::{Error, FromField, FromVariant, ToTokens};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DataEnum, DataStruct, DeriveInput, Ident};

use crate::proto_attrs::{IntoProto, IntoProtoFieldInfo, IntoProtoVariantInfo};
use crate::utils::{extract_type_from_option, extract_type_from_vec};

pub(crate) fn expand_into_proto(
    info: IntoProto,
    input: DeriveInput,
) -> Result<TokenStream, Error> {
    let name = input.ident;

    // match if this is a struct or an enum
    match input.data {
        | syn::Data::Struct(data) => expand_into_proto_struct(info, name, data),
        | syn::Data::Enum(data) => expand_into_proto_enum(info, name, data),
        | _ => panic!("IntoProto can only be derived for structs and enums"),
    }
}

fn expand_into_proto_struct(
    info: IntoProto,
    struct_name: Ident,
    struct_data: DataStruct,
) -> Result<TokenStream, Error> {
    let target_type = info.into;
    let mut fields = Vec::with_capacity(struct_data.fields.len());
    for field in struct_data.fields {
        let field_name = field.ident.clone().unwrap();
        let field_info = IntoProtoFieldInfo::from_field(&field)?;
        // We automatically skip non-public fields and fields that start with
        // '_' as both indicate that the field is not part of the public
        // API.
        //
        // pub(X). aka 'restricted' will still be included.
        if field_info.skip
            || field_name.to_string().starts_with('_')
            || field.vis == syn::Visibility::Inherited
        {
            continue;
        }
        // We use the same field name as the protobuf field name unless the user
        // specifies a different name via `#[into_proto(into = "foo"`)]`
        // attribute.
        let target_name = field_info.into.unwrap_or_else(|| field_name.clone());
        let span = field.span();
        // We are constructing the right-hand-side of the value
        // e.g.
        // timezone: <this>
        if field_info.map_fn.is_some() && field_info.map_method.is_some() {
            return Err(Error::custom(
                "Cannot specify both `map` and `map_method`",
            )
            .with_span(&span));
        }

        let mapper = if let Some(ref mapper) = field_info.map_fn {
            quote! {
                |d| #mapper(&d)
            }
        } else if let Some(ref mapper) = field_info.map_method {
            quote! {
                |d| d.#mapper()
            }
        } else {
            quote! {
                Into::into
            }
        };

        // cases we need to care about
        // - Option<T>
        // - Vec<T>
        let rhs = if let Some(inner_ty) = extract_type_from_option(&field.ty) {
            // Option<T>
            let span = inner_ty.span();
            // if the field is Option<T> but marked required, we unwrap.
            if field_info.required {
                quote_spanned! { span =>
                    value.#field_name.expect(
                        format!(
                            "Expected {} to have value, this field is marked as `required`",
                                stringify!(#field_name)
                        ).as_str()
                    )
                }
            } else {
                quote_spanned! { span =>
                    value.#field_name.map(#mapper)
                }
            }
        } else if extract_type_from_vec(&field.ty).is_some() {
            // A Vec<T>
            quote_spanned! { span =>
                value.#field_name.into_iter().map(#mapper).collect()
            }
        } else if let Some(method) = field_info.map_method {
            // A method that returns a type that implements Into<protobuf_type>
            quote_spanned! { span =>
                value.#field_name.#method().into()
            }
        } else if let Some(func) = field_info.map_fn {
            // A function that takes a reference to the value and returns a
            // type that implements `Into<protobuf_type>`
            quote_spanned! { span =>
                #func(&value.#field_name).into()
            }
        } else {
            // Everything else.
            quote_spanned! { span =>
                value.#field_name.into()
            }
        };

        let tok = quote_spanned! { span =>
                #target_name: #rhs,
        };
        fields.push(tok);
    }

    let tokens = quote! {
    #[automatically_derived]
    #[allow(clippy::all)]
    impl ::std::convert::From<#struct_name> for #target_type {
                fn from(value: #struct_name) -> Self {
                    Self {
                        #(#fields)*
                    }

                }
        }
    };

    Ok(tokens)
}

fn expand_into_proto_enum(
    info: IntoProto,
    enum_name: Ident,
    enum_data: DataEnum,
) -> Result<TokenStream, Error> {
    if enum_data.variants.is_empty() {
        return Err(Error::custom("Cannot derive IntoProto for an empty enum"));
    }

    // We cheat by looking at the first variant to determine whether this is a
    // unit-only enum or not. Ideally, we should fail if there is a mix of unit
    // and unnamed.
    let variant = enum_data.variants.first().unwrap();
    if let syn::Fields::Unit = variant.fields {
        expand_into_proto_unit_only_enum(info, enum_name, enum_data)
    } else {
        expand_into_proto_non_unit_enum(info, enum_name, enum_data)
    }
}

fn expand_into_proto_unit_only_enum(
    info: IntoProto,
    enum_name: Ident,
    enum_data: DataEnum,
) -> Result<TokenStream, Error> {
    let target_type_path = info.into;
    let mut variants: Vec<_> = Vec::with_capacity(enum_data.variants.len());
    for variant in enum_data.variants {
        let variant_name = &variant.ident;
        let variant_info = IntoProtoVariantInfo::from_variant(&variant)?;
        if variant_info.skip {
            continue;
        }
        let target_variant_name =
            variant_info.into.unwrap_or_else(|| variant_name.clone());

        let span = variant.span();
        let tok = quote_spanned! { span =>
            #enum_name::#variant_name => #target_type_path::#target_variant_name,
        };
        variants.push(tok);
    }

    // unit-only enum
    let tokens = quote! {
        #[automatically_derived]
        #[allow(clippy::all)]
        impl ::std::convert::From<#enum_name> for #target_type_path {
            fn from(value: #enum_name) -> Self {
                match value {
                    #(#variants)*
                }
            }
        }

        // Provides a conversion from our enum to i32's repr of the target proto
        #[automatically_derived]
        #[allow(clippy::all)]
        impl ::std::convert::From<#enum_name> for i32 {
            fn from(value: #enum_name) -> Self {
               let proto: #target_type_path = ::std::convert::Into::into(value);
               proto as i32
            }
        }
    };
    return Ok(tokens);
}

fn expand_into_proto_non_unit_enum(
    info: IntoProto,
    enum_name: Ident,
    enum_data: DataEnum,
) -> Result<TokenStream, Error> {
    let target_type_path = info.into;
    // * Non unit enums *
    // let's figure out the oneof name if it wasn't set in attributes.
    // Prost has a funny way to encode a message with oneof. let's assume a
    // message like this: ```
    // message Foo { --> The module name created but in snake_case.
    //   oneof bar { --> The type of the created enum but in PascalCase.
    //     Buff baz = 1;
    //     Stuff maz = 2;
    //  }
    // }
    //
    //
    // `some_prefix::Foo` is our target (`into=<>`), however, prost will create
    // a module `foo` with `Bar` as enum holding the variants. our variants will
    // be
    //
    // ```
    // some_prefix::foo::Bar::Baz
    // some_prefix::foo::Bar::Maz
    // ```
    //
    // In our case, we assume that our enum variants already match the target
    // variant ident (unless `into=` is passed in `into_proto` attribute.
    // However, we need to craft the path from the input target_type
    // (`Foo`). We assume `oneof`
    let mut target_path_segments = target_type_path.segments.clone();

    // `Foo` (target_path_segments is now `some_prefix::`)
    let target_path_tail_type = target_path_segments.pop().unwrap();
    // `foo`
    let target_tail_type_snakecase = Ident::new(
        &to_snake_case(&target_path_tail_type.value().ident.to_string()),
        Span::call_site(),
    );

    // `bar` (or `foo` by default)
    let oneof_ident = info
        .oneof
        .unwrap_or_else(|| target_tail_type_snakecase.clone());
    // `Bar`
    let oneof_ident_pascal =
        format_ident!("{}", to_pascal_case(&oneof_ident.to_string()));

    // `some_prefix::foo::Bar`
    let span = target_type_path.span();
    let fully_qualified_oneof_type = quote_spanned! { span =>
        #target_path_segments #target_tail_type_snakecase::#oneof_ident_pascal
    };

    let mut variants: Vec<_> = Vec::with_capacity(enum_data.variants.len());
    for variant in enum_data.variants {
        let variant_name = &variant.ident;
        let variant_info = IntoProtoVariantInfo::from_variant(&variant)?;
        if variant_info.skip {
            continue;
        }
        let target_variant_name =
            variant_info.into.unwrap_or_else(|| variant_name.clone());
        let span = variant.span();

        let v = match variant.fields {
            | syn::Fields::Named(_) => {
                return Err(darling::Error::unsupported_shape(
                    "IntoProto can only be derived for enums with unnamed or \
                     unit variants",
                ));
            }
            | syn::Fields::Unit => {
                return Err(darling::Error::unsupported_shape(
                    "IntoProto cannot be used with enums with mixed unit and \
                     non-unit variants ",
                ));
            }
            | syn::Fields::Unnamed(_) => {
                quote_spanned! { span =>
                    #enum_name::#variant_name(v) => {
                        #fully_qualified_oneof_type::#target_variant_name(v.into())
                    }
                }
            }
        };
        variants.push(v);
    }

    let target_type_path_str = target_type_path.to_token_stream().to_string();

    if info.non_exhaustive {
        variants.push(quote! {
            _e => {
                // Missing variants.
                panic!("Attempting to convert variant of `{:?}` with no match in the target type `{}`",
                       _e, #target_type_path_str);
            }
        });
    }

    let tokens = quote! {
    #[automatically_derived]
    impl ::std::convert::From<#enum_name> for #target_type_path {
                fn from(value: #enum_name) -> Self {
                    let o = match value {
                        #(#variants)*
                    };
                    Self {
                        #oneof_ident: ::std::option::Option::Some(o),
                    }
                }
        }
    };

    Ok(tokens)
}

fn to_snake_case(input: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in input.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

fn to_pascal_case(input: &str) -> String {
    let mut pascal = String::new();
    let mut capitalize = true;
    for ch in input.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            pascal.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            pascal.push(ch);
        }
    }
    pascal
}
