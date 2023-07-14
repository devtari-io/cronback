use darling::util::SpannedValue;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

use crate::attributes::{Direction, ProtoFieldInfo, Skip};
use crate::utils::{
    extract_inner_type_from_container,
    option_segment,
    vec_segment,
};

impl ProtoFieldInfo {
    pub(crate) fn gen_tokens(
        self,
        direction: Direction,
    ) -> darling::Result<TokenStream> {
        let span = self.ident.span();

        // Some validation
        if *self.map_from_by_ref && self.map_from_proto.is_none() {
            return Err(darling::Error::custom(
                "map_from_by_ref can only be used with map_from_proto",
            ));
        }

        if *self.map_into_by_ref && self.map_into_proto.is_none() {
            return Err(darling::Error::custom(
                "map_into_by_ref can only be used with map_into_proto",
            ));
        }

        // We use the same field name as the protobuf field name unless the user
        // specifies a different name via `#[into_proto(name = "foo"`)]`
        // attribute.
        let our_name = &self.ident();
        let proto_name = &self.name.as_ref().unwrap_or(our_name);

        let dest_field = if direction == Direction::IntoProto {
            proto_name
        } else {
            our_name
        };

        let mut rhs_value_tok = if direction == Direction::IntoProto {
            quote_spanned! { span =>
                value.#our_name
            }
        } else {
            quote_spanned! { span =>
                value.#proto_name
            }
        };

        // How do we map the value?
        // - Option<T>:
        //  - We map the inner value.
        //  - IntoProto + required: target is not option. we unwrap.
        //    (.map()).unwrap()
        //  - FromProto + required: input is not Option. Adding `required` does
        //    nothing since into() already handles the T -> Option<T>
        //    conversion.
        //
        // - Vec<T>:
        //  - We map each element
        //  - IntoProto + required: .into() should handle it.
        //  - FromProto + required: our_name: incoming.unwrap()
        //
        // - always add .into() after mapping.

        // Primary cases we need to take care of:
        //  - Skipped
        //  - Option<T>
        //  - Vec<T>
        //  - Everything else
        //
        if self.is_skipped() {
            // skip this field if it is marked with #[proto(skip)]
            let tok = match direction {
                | Direction::IntoProto => TokenStream::new(),
                | Direction::FromProto => {
                    // FromProto: By skipping we initialize with Default value.
                    quote_spanned! { span =>
                        #dest_field: ::std::default::Default::default(),
                    }
                }
            };
            return Ok(tok);
        }

        let option_type =
            extract_inner_type_from_container(&self.ty, option_segment);
        let vec_type = extract_inner_type_from_container(&self.ty, vec_segment);

        // 1. Do we need to unwrap the input before processing? We do that if
        // the field is    `required` and our local type is not `Option<T>` when
        // converting from proto to rust.

        if option_type.is_none()
            && direction == Direction::FromProto
            && self.required
        {
            rhs_value_tok = quote_spanned! { span =>
                #rhs_value_tok.unwrap()
            };
        }

        if let Some(_inner_ty) = option_type {
            // Option<T>
            // There is specific case we need to handle. When converting a
            // 'required' field from Proto to Option<T>, we run wrap the input
            // into an option (`Some`) before mapping.
            if direction == Direction::FromProto && self.required {
                let span = self.required.span();
                rhs_value_tok = quote_spanned! { span =>
                    Some(#rhs_value_tok)
                };
            }

            let mapper = self
                .wrap_with_mapper(direction, quote! { v })
                .map(|mapper| {
                    quote_spanned! { span =>
                        |v| #mapper
                    }
                })
                // If there is no mapper, we just map the inner value with any
                // existing Into impl.
                .unwrap_or_else(|| {
                    quote_spanned! { span =>
                        Into::into
                    }
                });

            rhs_value_tok = quote_spanned! { span =>
                #rhs_value_tok.map(#mapper)
            };

            // We unwrap after map if our target proto type not Option
            if self.required && direction == Direction::IntoProto {
                rhs_value_tok = quote_spanned! { span =>
                    #rhs_value_tok.unwrap()
                };
            }
        } else if let Some(_inner_ty) = vec_type {
            // A Vec<T>
            let mapper = self
                .wrap_with_mapper(direction, quote! { v })
                .map(|mapper| {
                    quote_spanned! { span =>
                            |v| #mapper
                    }
                })
                // If there is no mapper, we just map the inner value with any
                // existing Into impl.
                .unwrap_or_else(|| {
                    quote_spanned! { span =>
                        Into::into
                    }
                });
            rhs_value_tok = quote_spanned! { span =>
                #rhs_value_tok.into_iter().map(#mapper).collect::<::std::vec::Vec<_>>()
            };
        } else {
            // Bare type
            rhs_value_tok = self
                .wrap_with_mapper(
                    direction,
                    quote_spanned! { span => #rhs_value_tok },
                )
                .unwrap_or(rhs_value_tok);
            // We need to .into()
            rhs_value_tok = quote_spanned! { span => #rhs_value_tok.into() };

            if self.required && direction == Direction::IntoProto {
                rhs_value_tok = quote_spanned! { span => Some(#rhs_value_tok) };
            }
        };

        Ok(quote_spanned! { span =>
                #dest_field: #rhs_value_tok,
        })
    }

    // Wraps input with mapper function. E.g. `mapper(input)` or
    // `mapper(&input)` depends on whether by_ref is set or not.
    fn wrap_with_mapper(
        &self,
        direction: Direction,
        input: TokenStream,
    ) -> Option<TokenStream> {
        fn gen_mapped_inner(
            by_ref: SpannedValue<bool>,
            mapper_path: &syn::Path,
            input: TokenStream,
        ) -> TokenStream {
            // do we have a built-in mapper?
            let span = by_ref.span();
            let opt_ref = if *by_ref {
                quote_spanned! { span => &}
            } else {
                quote! {}
            };
            let span = mapper_path.span();
            quote_spanned! { span =>
                #mapper_path(#opt_ref #input)
            }
        }

        match direction {
            | Direction::IntoProto if self.map_into_proto.is_some() => {
                Some(gen_mapped_inner(
                    self.map_into_by_ref,
                    self.map_into_proto.as_ref().unwrap(),
                    input,
                ))
            }
            | Direction::FromProto if self.map_from_proto.is_some() => {
                Some(gen_mapped_inner(
                    self.map_from_by_ref,
                    self.map_from_proto.as_ref().unwrap(),
                    input,
                ))
            }
            | _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use darling::error::Accumulator;
    use darling::FromField;
    use pretty_assertions::assert_eq;
    use syn::parse::Parser;

    use super::*;

    #[track_caller]
    fn gen_tokens_test_helper(
        field: ProtoFieldInfo,
        direction: Direction,
        expected: TokenStream,
    ) -> darling::Result<()> {
        let mut acc = Accumulator::default();
        let actual = acc.handle(field.gen_tokens(direction));
        let o = acc.finish();
        if o.is_err() {
            return o;
        }
        let actual = actual.unwrap();
        assert_eq!(expected.to_string(), actual.to_string());
        Ok(())
    }

    #[track_caller]
    fn gen_tokens_test_helper_into(
        field: ProtoFieldInfo,
        expected: TokenStream,
    ) -> darling::Result<()> {
        gen_tokens_test_helper(field, Direction::IntoProto, expected)
    }

    #[track_caller]
    fn gen_tokens_test_helper_from(
        field: ProtoFieldInfo,
        expected: TokenStream,
    ) -> darling::Result<()> {
        gen_tokens_test_helper(field, Direction::FromProto, expected)
    }

    fn field_from_quote(quote: TokenStream) -> darling::Result<ProtoFieldInfo> {
        let field: syn::Field = syn::Field::parse_named.parse2(quote).unwrap();
        ProtoFieldInfo::from_field(&field)
    }

    #[test]
    fn gen_tokens_skipped() -> darling::Result<()> {
        // non-pub
        {
            let field_info = field_from_quote(quote! { foo: u32 })?;

            // no tokens in IntoProto
            gen_tokens_test_helper_into(field_info.clone(), quote! {})?;

            // default in FromProto
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: ::std::default::Default::default(), },
            )?;
        }

        // public but starts with _
        {
            let field_info = field_from_quote(quote! { pub _foo: u32 })?;

            // no tokens in IntoProto
            gen_tokens_test_helper_into(field_info.clone(), quote! {})?;

            // default in FromProto
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { _foo: ::std::default::Default::default(), },
            )?;
        }

        // public but explicity skipped
        {
            let field_info =
                field_from_quote(quote! { #[proto(skip)] pub foo: u32 })?;

            // no tokens in IntoProto
            gen_tokens_test_helper_into(field_info.clone(), quote! {})?;

            // default in FromProto
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: ::std::default::Default::default(), },
            )?;
        }
        Ok(())
    }

    #[test]
    fn gen_tokens_bare() -> darling::Result<()> {
        // No mapping, no wrapping.
        {
            let field_info = field_from_quote(quote! { pub foo: u32 })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.into(), },
            )?;
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.into(), },
            )?;
        }

        // #[proto(required)] bare type
        {
            let field_info = field_from_quote(quote! {
                #[proto(required)]
                pub foo: u32
            })?;

            // We are not Option<T> but the target is likely is (hence
            // `required`) We need to explicitly wrap into Some() after `into()`
            // to convert T -> Option<B>
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: Some(value.foo.into()), },
            )?;
            // We unwrap only proto -> rust.
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.unwrap().into(), },
            )?;
        }

        // #[proto(map_from_proto)] bare type
        {
            // map by value (default)
            let field_info = field_from_quote(quote! {
                #[proto(map_from_proto="String::from")]
                pub foo: String
            })?;

            // no effect on into
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.into(), },
            )?;
            // We map only proto -> rust. by value.
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: String::from(value.foo).into(), },
            )?;

            // by reference
            let field_info = field_from_quote(quote! {
                #[proto(map_from_proto="String::from", map_from_by_ref)]
                pub foo: String
            })?;

            // We map only proto -> rust. by value.
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: String::from(&value.foo).into(), },
            )?;
        }

        // #[proto(map_into_proto)] bare type
        {
            // by value (default)
            let field_info = field_from_quote(quote! {
                #[proto(map_into_proto="String::from")]
                pub foo: u32
            })?;

            // no effect on from
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.into(), },
            )?;
            // We map only rust -> proto. by value.
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: String::from(value.foo).into(), },
            )?;

            // by reference
            let field_info = field_from_quote(quote! {
                #[proto(map_into_proto="String::from", map_into_by_ref)]
                pub foo: String
            })?;

            // We map only proto -> rust. by value.
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: String::from(&value.foo).into(), },
            )?;
        }

        // #[proto(map_into_proto)] bare type with rename
        {
            // by value (default)
            let field_info = field_from_quote(quote! {
                #[proto(name = "bar", map_into_proto="String::from")]
                pub foo: u32
            })?;

            // FromProto simple rename
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.bar.into(), },
            )?;
            // We map and rename the rust -> proto.
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { bar: String::from(value.foo).into(), },
            )?;
        }

        Ok(())
    }

    #[test]
    fn gen_tokens_option() -> darling::Result<()> {
        // No mapping, no wrapping.
        {
            let field_info = field_from_quote(quote! { pub foo: Option<u32> })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.map(Into::into), },
            )?;
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.map(Into::into), },
            )?;
        }

        // #[proto(required)] Option<T> type
        {
            let field_info = field_from_quote(quote! {
                #[proto(required)]
                pub foo: Option<u32>
            })?;

            // In IntoProto, we assume that the target is not option, so we need
            // to unwrap.
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.map(Into::into).unwrap(), },
            )?;
            // In FromProto, we wrap the value in Some(v) and map it.
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: Some(value.foo).map(Into::into), },
            )?;
        }
        Ok(())
    }

    #[test]
    fn gen_tokens_mapped_option() -> darling::Result<()> {
        // Map an Option IntoProto
        {
            let field_info = field_from_quote(quote! {
               #[proto(map_into_proto="String::from")]
                pub foo: Option<u32>
            })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.map(|v| String::from(v)), },
            )?;
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.map(Into::into), },
            )?;
        }

        // Map an Option FromProto
        {
            let field_info = field_from_quote(quote! {
               #[proto(map_from_proto="String::from")]
                pub foo: Option<u32>
            })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.map(Into::into), },
            )?;

            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.map(|v| String::from(v)), },
            )?;
        }

        // Map an Option FromProto by reference
        {
            let field_info = field_from_quote(quote! {
               #[proto(map_from_proto="String::from", map_from_by_ref)]
                pub foo: Option<u32>
            })?;

            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.map(|v| String::from(&v)), },
            )?;
        }

        // Complex: Map a required Option Into/FromProto
        {
            let field_info = field_from_quote(quote! {
               #[proto(
                   required,
                   map_from_proto="String::from",
                   map_from_by_ref,
                   map_into_proto="AnotherType::from",
                   map_into_by_ref
                   )
               ]
                pub foo: Option<u32>
            })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.map(|v| AnotherType::from(&v)).unwrap(), },
            )?;

            // What's coming from proto is _not_ an Option, we cannot apply
            // .map()
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: Some(value.foo).map(|v| String::from(&v)), },
            )?;
        }
        Ok(())
    }

    #[test]
    fn gen_tokens_vec() -> darling::Result<()> {
        // Vec with no mapping
        {
            let field_info = field_from_quote(quote! {
                pub foo: Vec<u32>
            })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.into_iter().map(Into::into).collect::<::std::vec::Vec<_>>(), },
            )?;
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.into_iter().map(Into::into).collect::<::std::vec::Vec<_>>(), },
            )?;
        }
        // Required Vec.
        {
            let field_info = field_from_quote(quote! {
                #[proto(required)]
                pub foo: Vec<u32>
            })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.into_iter().map(Into::into).collect::<::std::vec::Vec<_>>(), },
            )?;
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.unwrap().into_iter().map(Into::into).collect::<::std::vec::Vec<_>>(), },
            )?;
        }
        Ok(())
    }

    #[test]
    fn gen_tokens_mapped_vec() -> darling::Result<()> {
        // Vec with mapping
        {
            let field_info = field_from_quote(quote! {
                #[proto(
                   map_from_proto="String::from",
                   map_into_proto="AnotherType::from",
                   )
                ]
                pub foo: Vec<u32>
            })?;

            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.into_iter().map(|v|
                AnotherType::from(v)).collect::<::std::vec::Vec<_>>(), },
            )?;
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.into_iter().map(|v|
                String::from(v)).collect::<::std::vec::Vec<_>>(), },
            )?;
        }
        // required Vec with mapping by ref
        {
            let field_info = field_from_quote(quote! {
                #[proto(
                   required,
                   map_from_proto="String::from",
                   // only from is by ref
                   map_from_by_ref,
                   map_into_proto="AnotherType::from",
                   )
                ]
                pub foo: Vec<u32>
            })?;

            // same as before. Some() wrapping happens by into().
            gen_tokens_test_helper_into(
                field_info.clone(),
                quote! { foo: value.foo.into_iter().map(|v|
                AnotherType::from(v)).collect::<::std::vec::Vec<_>>(), },
            )?;

            // We need to unwrap before iterating over the vector. We assume
            // here that the input coming from Proto is Option<Vec<T>>
            gen_tokens_test_helper_from(
                field_info.clone(),
                quote! { foo: value.foo.unwrap().into_iter().map(|v|
                String::from(&v)).collect::<::std::vec::Vec<_>>(), },
            )?;
        }
        Ok(())
    }
}
