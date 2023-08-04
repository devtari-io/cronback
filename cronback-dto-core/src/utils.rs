/// Heavily influenced/copied from https://stackoverflow.com/questions/55271857/how-can-i-get-the-t-from-an-optiont-when-using-syn
use syn::{GenericArgument, Path, PathArguments};

pub(crate) fn extract_generic_type_segment<'a>(
    path: &'a Path,
    possible_matches: &'static [&str],
) -> Option<&'a syn::PathSegment> {
    let idents_of_path =
        path.segments.iter().fold(String::new(), |mut acc, v| {
            acc.push_str(&v.ident.to_string());
            acc.push('|');
            acc
        });

    possible_matches
        .iter()
        .find(|s| &idents_of_path == *s)
        .and_then(|_| path.segments.last())
}

pub(crate) fn option_segment(path: &syn::Path) -> Option<&syn::PathSegment> {
    static OPTION: &[&str] =
        &["Option|", "std|option|Option|", "core|option|Option|"];
    extract_generic_type_segment(path, OPTION)
}

pub(crate) fn vec_segment(path: &syn::Path) -> Option<&syn::PathSegment> {
    static VECTOR: &[&str] = &["Vec|", "std|vec|Vec|", "alloc|vec|Vec|"];
    extract_generic_type_segment(path, VECTOR)
}

pub(crate) fn map_segment(path: &syn::Path) -> Option<&syn::PathSegment> {
    static MAP: &[&str] = &["HashMap|", "std|collections|HashMap|"];
    extract_generic_type_segment(path, MAP)
}

fn extract_type_path(ty: &syn::Type) -> Option<&syn::Path> {
    match *ty {
        | syn::Type::Path(ref typepath) if typepath.qself.is_none() => {
            Some(&typepath.path)
        }
        | _ => None,
    }
}

pub(crate) fn extract_inner_type_from_container(
    ty: &syn::Type,
    extractor: fn(&syn::Path) -> Option<&syn::PathSegment>,
) -> Option<&syn::Type> {
    extract_type_path(ty)
        .and_then(extractor)
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

pub(crate) fn to_snake_case(input: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in input.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

pub(crate) fn to_pascal_case(input: &str) -> String {
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
