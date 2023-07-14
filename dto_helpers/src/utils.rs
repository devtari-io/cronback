/// Heavily influenced/copied from https://stackoverflow.com/questions/55271857/how-can-i-get-the-t-from-an-optiont-when-using-syn

pub(crate) fn extract_type_from_option(ty: &syn::Type) -> Option<&syn::Type> {
    use syn::{GenericArgument, Path, PathArguments, PathSegment};

    fn extract_type_path(ty: &syn::Type) -> Option<&Path> {
        match *ty {
            | syn::Type::Path(ref typepath) if typepath.qself.is_none() => {
                Some(&typepath.path)
            }
            | _ => None,
        }
    }

    fn extract_option_segment(path: &Path) -> Option<&PathSegment> {
        let idents_of_path =
            path.segments.iter().fold(String::new(), |mut acc, v| {
                acc.push_str(&v.ident.to_string());
                acc.push('|');
                acc
            });
        vec!["Option|", "std|option|Option|", "core|option|Option|"]
            .into_iter()
            .find(|s| idents_of_path == *s)
            .and_then(|_| path.segments.last())
    }

    extract_type_path(ty)
        .and_then(extract_option_segment)
        .and_then(|path_seg| {
            let type_params = &path_seg.arguments;
            // It should have only on angle-bracketed param ("<String>"):
            match *type_params {
                | PathArguments::AngleBracketed(ref params) => {
                    params.args.first()
                }
                | _ => None,
            }
        })
        .and_then(|generic_arg| {
            match *generic_arg {
                | GenericArgument::Type(ref ty) => Some(ty),
                | _ => None,
            }
        })
}

pub(crate) fn extract_type_from_vec(ty: &syn::Type) -> Option<&syn::Type> {
    use syn::{GenericArgument, Path, PathArguments, PathSegment};

    fn extract_type_path(ty: &syn::Type) -> Option<&Path> {
        match *ty {
            | syn::Type::Path(ref typepath) if typepath.qself.is_none() => {
                Some(&typepath.path)
            }
            | _ => None,
        }
    }

    fn extract_vec_segment(path: &Path) -> Option<&PathSegment> {
        let idents_of_path =
            path.segments.iter().fold(String::new(), |mut acc, v| {
                acc.push_str(&v.ident.to_string());
                acc.push('|');
                acc
            });
        vec!["Vec|", "std|vec|Vec|", "alloc|vec|Vec|"]
            .into_iter()
            .find(|s| idents_of_path == *s)
            .and_then(|_| path.segments.last())
    }

    extract_type_path(ty)
        .and_then(extract_vec_segment)
        .and_then(|path_seg| {
            let type_params = &path_seg.arguments;
            // It should have only on angle-bracketed param ("<String>"):
            match *type_params {
                | PathArguments::AngleBracketed(ref params) => {
                    params.args.first()
                }
                | _ => None,
            }
        })
        .and_then(|generic_arg| {
            match *generic_arg {
                | GenericArgument::Type(ref ty) => Some(ty),
                | _ => None,
            }
        })
}
