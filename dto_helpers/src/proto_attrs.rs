use darling::{FromDeriveInput, FromField, FromVariant};
use syn::{Ident, Path};

/* IntoProto */

// Attributes for struct/enum level #[into_proto(...)]
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(into_proto),
    supports(struct_named, enum_newtype, enum_unit)
)]
pub struct IntoProto {
    pub ident: Ident,
    pub into: Path,
    pub oneof: Option<Ident>,
    #[darling(default)]
    // If true, the generated match will include a default arm.
    pub non_exhaustive: bool,
}

// Attributes for enum-variant level #[into_proto(...)]
#[derive(Default, Debug, FromVariant)]
#[darling(attributes(into_proto), default)]
pub struct IntoProtoVariantInfo {
    pub into: Option<Ident>,
    pub skip: bool,
}

// Attributes for struct-field level #[into_proto(...)]
#[derive(Debug, Default, FromField)]
#[darling(attributes(into_proto), default)]
pub struct IntoProtoFieldInfo {
    pub skip: bool,
    pub into: Option<Ident>,
    pub map_fn: Option<Path>,
    // Applies the method to the field value before conversion.
    pub map_method: Option<Path>,
    // If the field is Option but the target is not, we unwrap()
    pub required: bool,
}

/* FromProto */
#[allow(dead_code)]
#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(from_proto),
    supports(struct_named, enum_newtype, enum_unit)
)]
pub struct FromProto {
    ident: Ident,
    pub from: String,
}
