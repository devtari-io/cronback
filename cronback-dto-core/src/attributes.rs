use std::fmt::{Display, Formatter};

use darling::util::SpannedValue;
use darling::{FromDeriveInput, FromField, FromVariant};
use syn::{Field, Variant};

#[derive(Debug, Clone)]
pub(crate) enum Direction<F, I> {
    FromProto(F),
    IntoProto(I),
}

impl<F, I> Direction<F, I>
where
    F: Clone + Clone,
    I: Clone + Clone,
{
    pub fn is_into(&self) -> bool {
        matches!(self, Direction::IntoProto(_))
    }

    pub fn is_from(&self) -> bool {
        matches!(self, Direction::FromProto(_))
    }

    pub fn with_variant(
        &self,
        variant: &Variant,
    ) -> darling::Result<Direction<FromProtoVariantInfo, IntoProtoVariantInfo>>
    {
        Ok(match self {
            | Direction::FromProto(_) => {
                Direction::FromProto(FromProtoVariantInfo::from_variant(
                    variant,
                )?)
            }
            | Direction::IntoProto(_) => {
                Direction::IntoProto(IntoProtoVariantInfo::from_variant(
                    variant,
                )?)
            }
        })
    }

    pub fn with_field(
        &self,
        field: &Field,
    ) -> darling::Result<Direction<FromProtoFieldInfo, IntoProtoFieldInfo>>
    {
        Ok(match self {
            | Direction::FromProto(_) => {
                Direction::FromProto(FromProtoFieldInfo::from_field(field)?)
            }
            | Direction::IntoProto(_) => {
                Direction::IntoProto(IntoProtoFieldInfo::from_field(field)?)
            }
        })
    }
}

impl<A, B> Display for Direction<A, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            | Direction::FromProto(_) => write!(f, "FromProto"),
            | Direction::IntoProto(_) => write!(f, "IntoProto"),
        }
    }
}

// Attributes for struct/enum level #[proto(...)]
#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(proto), supports(struct_named, enum_newtype, enum_unit))]
pub(crate) struct ProtoInfo {
    pub ident: syn::Ident,
    pub target: syn::Path,
    pub oneof: Option<syn::Ident>,
    #[darling(default)]
    // If true, the generated match will include a default arm.
    pub non_exhaustive: SpannedValue<bool>,
}

// Attributes for struct/enum level #[from_proto(...)]
#[derive(Debug, Clone, FromDeriveInput)]
#[darling(
    attributes(from_proto),
    supports(struct_named, enum_newtype, enum_unit)
)]
pub(crate) struct FromProtoInfo {
    // Reserved for future use.
}

// Attributes for struct/enum level #[into_proto(...)]
#[derive(Debug, Clone, FromDeriveInput)]
#[darling(
    attributes(into_proto),
    supports(struct_named, enum_newtype, enum_unit)
)]
pub(crate) struct IntoProtoInfo {
    // Reserved for future use.
}

// Attributes for enum-variant level #[proto(...)]
// This one is used for common attributes across from and into.
#[derive(Debug, Clone, FromVariant)]
#[darling(attributes(proto))]
pub(crate) struct ProtoVariantInfo {
    // automatically populated by darling
    pub ident: syn::Ident,
    pub fields: darling::ast::Fields<ProtoEnumFieldInfo>,
    // our proto variant attributes
    #[darling(default)]
    pub name: Option<syn::Ident>,
    #[darling(default)]
    pub skip: bool,
}
// Attributes for enum-variant level #[from_proto(...)]
#[derive(Debug, Clone, FromVariant)]
#[darling(attributes(from_proto))]
pub(crate) struct FromProtoVariantInfo {
    // Reserved for future use.
}
// Attributes for enum-variant level #[into_proto(...)]
#[derive(Debug, Clone, FromVariant)]
#[darling(attributes(into_proto))]
pub(crate) struct IntoProtoVariantInfo {
    // Reserved for future use.
}

#[derive(Debug, Clone, FromField)]
#[darling(attributes(proto))]
pub(crate) struct ProtoEnumFieldInfo {
    // Reserved for future use.
}

// Attributes for struct-field level #[proto(...)]
#[derive(Debug, Clone, FromField)]
#[darling(attributes(proto))]
pub(crate) struct ProtoFieldInfo {
    // automatically populated by darling
    pub ident: Option<syn::Ident>,
    pub vis: syn::Visibility,
    pub ty: syn::Type,

    // our proto field attributes
    #[darling(default)]
    pub skip: bool,
    #[darling(default)]
    pub name: Option<syn::Ident>,
    #[darling(default)]
    pub required: bool,
}

impl ProtoFieldInfo {
    // We only support structs with named fields (no tuples) so we can safely
    // unwrap ident.
    pub fn ident(&self) -> &syn::Ident {
        self.ident.as_ref().unwrap()
    }
}

// Attributes for struct-field level #[from_proto(...)]
#[derive(Debug, Clone, FromField)]
#[darling(attributes(from_proto))]
pub(crate) struct FromProtoFieldInfo {
    #[darling(default)]
    // Always set the value to None (if must be Option<T>) in FromProto
    // conversion, effectively making this a read-only field.
    pub always_none: bool,

    #[darling(default)]
    pub map: Option<syn::Path>,
    #[darling(default)]
    pub map_by_ref: SpannedValue<bool>,
}

// Attributes for struct-field level #[into_proto(...)]
#[derive(Debug, Clone, FromField)]
#[darling(attributes(into_proto))]
pub(crate) struct IntoProtoFieldInfo {
    #[darling(default)]
    pub map: Option<syn::Path>,
    #[darling(default)]
    pub map_by_ref: SpannedValue<bool>,
}

pub(crate) trait Skip {
    fn is_skipped(&self) -> bool;
}

impl Skip for ProtoFieldInfo {
    // We automatically skip non-public fields and fields that start with
    // '_' as both indicate that the field is not part of the public
    // API.
    //
    // pub(X). aka 'restricted' will still be included.
    fn is_skipped(&self) -> bool {
        self.skip
            || self.ident().to_string().starts_with('_')
            || self.vis == syn::Visibility::Inherited
    }
}

impl Skip for ProtoVariantInfo {
    fn is_skipped(&self) -> bool {
        self.skip
    }
}

// Experiment
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(prost))]
pub(crate) struct ProstMessageInto {
    pub ident: syn::Ident,
}

#[allow(unused)]
#[derive(Debug, Clone, FromField)]
#[darling(attributes(prost), forward_attrs(doc), allow_unknown_fields)]
pub(crate) struct ProstFieldInfo {
    // automatically populated by darling
    pub ident: Option<syn::Ident>,
    pub vis: syn::Visibility,
    pub ty: syn::Type,

    pub attrs: Vec<syn::Attribute>,
    is_enumeration: Option<syn::Path>,
    #[darling(default, rename = "message")]
    is_message: bool,
    #[darling(default, rename = "optional")]
    is_optional: bool,
}
