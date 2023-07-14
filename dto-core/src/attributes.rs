use std::fmt::{Display, Formatter};

use darling::util::SpannedValue;
use darling::{FromDeriveInput, FromField, FromVariant};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Direction {
    FromProto,
    IntoProto,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            | Direction::FromProto => write!(f, "FromProto"),
            | Direction::IntoProto => write!(f, "IntoProto"),
        }
    }
}

// Attributes for struct/enum level #[proto(...)]
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(proto), supports(struct_named, enum_newtype, enum_unit))]
pub(crate) struct ProtoInfo {
    pub ident: syn::Ident,
    pub target: syn::Path,
    pub oneof: Option<syn::Ident>,
    #[darling(default)]
    // If true, the generated match will include a default arm.
    pub non_exhaustive: SpannedValue<bool>,
}

// Attributes for enum-variant level #[proto(...)]
#[derive(Debug, FromVariant)]
#[cfg_attr(test, derive(Clone))]
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

#[derive(Debug, FromField)]
#[cfg_attr(test, derive(Clone))]
#[darling(attributes(proto))]
pub(crate) struct ProtoEnumFieldInfo {
    // Reserved for future use.
}

// Attributes for struct-field level #[proto(...)]
#[derive(Debug, FromField)]
#[cfg_attr(test, derive(Clone))]
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
    pub map_from_proto: Option<syn::Path>,
    #[darling(default)]
    pub map_from_by_ref: SpannedValue<bool>,
    #[darling(default)]
    pub map_into_proto: Option<syn::Path>,
    #[darling(default)]
    pub map_into_by_ref: SpannedValue<bool>,

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
