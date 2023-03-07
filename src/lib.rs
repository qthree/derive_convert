use std::collections::HashMap;

use proc_macro::{TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, DataStruct, DataEnum, Attribute, Meta, NestedMeta, Path, Lit, Fields, Type, MetaList, Expr, MetaNameValue, parse::Parse};
use proc_macro2::{TokenStream as TokenStream2, Ident};

#[proc_macro_derive(TryFrom, attributes(try_from))]
pub fn derive_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if !input.generics.params.is_empty() {
        unimplemented!("Generics are not supported!");
    }
    let container_attrs = parse_container_attrs(&input.attrs).expect("Parse attributes to find paths from `try_from`");

    match &input.data {
        Data::Struct(data) => derive_convert_struct(&container_attrs, &input.ident, data),
        Data::Enum(data) => derive_convert_enum(&container_attrs, &input.ident, data),
        Data::Union(_) => unimplemented!("Unions are not supported!"),
    }.into()
}

fn derive_convert_struct(container_attrs: &ContainerAttrs, to: &Ident, data: &DataStruct) -> TokenStream2 {
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => unimplemented!("Only structs with named fields are supported")
    };
    struct FieldOptions<'a> {
        name: &'a Ident,
        attrs: FieldAttrs,
    }
    let fields: Vec<_> = fields.into_iter().map(|field| {
        let attrs = parse_field_attrs(&field.attrs).expect("Parse attributes to find options for `try_from`");
        attrs.check(container_attrs);
        FieldOptions{name: field.ident.as_ref().unwrap(), attrs}
    }).collect();

    let ContainerAttrs{from, err_ty} = container_attrs;

    from.into_iter().map(|(from_key, from_value)| {
        let lines = fields.iter().map(|field| {
            let name = field.name;
            match field.attrs.map_for(from_key) {
                Map::Lit(lit) | Map::With(lit) => {
                    quote!(#name: (#lit)(value.#name)?,)
                }
                Map::TryInto => {
                    quote!(#name: value.#name.try_into()?,)
                }
                Map::Defualt => {
                    quote!(#name: Default::default(),)
                },
            }
        });
        quote! {
            impl std::convert::TryFrom<#from_value> for #to {
                type Error = #err_ty;

                fn try_from(value: #from_value) -> Result<#to, Self::Error> {
                    Ok(Self {
                        #(
                            #lines
                        )*
                    })
                }
            }
        }
    }).collect()
}

fn derive_convert_enum(ContainerAttrs{from, err_ty}: &ContainerAttrs, to: &Ident, data: &DataEnum) -> TokenStream2 {
    let variants: Vec<_> = data.variants.iter().map(|var| {
        if !var.fields.is_empty() {
            unimplemented!("Only C-like enums are supported")
        }
        &var.ident
    }).collect();
    from.values().map(|from| {
        quote! {
            impl std::convert::TryFrom<#from> for #to {
                type Error = #err_ty;

                fn try_from(value: #from) -> Result<#to, Self::Error> {
                    Ok(match value {
                        #(
                            #from::#variants => #to::#variants,
                        )*
                    })
                }
            }
        }
    }).collect()
}

struct ContainerAttrs {
    from: HashMap<Ident, Type>,
    err_ty: Type,
}

fn parse_container_attrs(attrs: &[Attribute]) -> Result<ContainerAttrs, ParseAttrsError> {
    let mut from = HashMap::new();
    let mut err_ty = None;
    let iter = attrs
        .into_iter()
        .filter(|attr| path_eq(&attr.path, "try_from"));

    for attr in iter {
        let meta = attr.parse_meta().map_err(|_| ParseAttrsError::ParseMeta)?;
        match meta {
            Meta::List(list) => {
                for nested in list.nested {
                    match nested {
                        NestedMeta::Meta(nested_meta) => {
                            match nested_meta {
                                Meta::NameValue(name_value) if path_eq(&name_value.path, "Error") => {
                                    let err: Type = name_value_parse(&name_value).ok_or(ParseAttrsError::UnsupportedErrLiteral)?;
                                    if let Some(_old_err) = err_ty.replace(err) {
                                        return Err(ParseAttrsError::DuplicateAttributes);
                                    }
                                }
                                Meta::NameValue(name_value) => {
                                    let key = name_value.path.get_ident().cloned().ok_or(ParseAttrsError::UnsupportedStructure)?;
                                    let map: Type = name_value_parse(&name_value).ok_or(ParseAttrsError::UnsupportedKeyLiteral)?;
                                    if let Some(_old_value) = from.insert(key, map) {
                                        return Err(ParseAttrsError::DuplicateAttributes);
                                    }
                                }
                                /*Meta::List(list) if path_eq(&list.path, "Err") && list.nested.len() == 1 => {
                                    match list.nested.into_iter().next().unwrap() {
                                        NestedMeta::Meta(Meta::Path(path)) => if let Some(_old_err) = err_ty.replace(path) {
                                            return Err(ParseAttrsError::DuplicateAttributes);
                                        },
                                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                                    }
                                }*/
                                /*Meta::List(list) if list.nested.len() == 1 => {
                                    match (list.path.get_ident().cloned(), list.nested.into_iter().next().unwrap()) {
                                        (Some(key), NestedMeta::Meta(Meta::Path(value))) => if let Some(_old_value) = from.insert(key, value) {
                                            return Err(ParseAttrsError::DuplicateAttributes);
                                        },
                                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                                    }
                                }*/
                                _ => return Err(ParseAttrsError::UnsupportedStructure),
                            }
                        }
                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                    }
                }
            }
            _ => return Err(ParseAttrsError::UnsupportedStructure),
        }
    }
    if from.is_empty() {
        return Err(ParseAttrsError::NoPaths)
    }
    if let Some(err_ty) = err_ty {
        Ok(ContainerAttrs{err_ty, from})
    } else {
        Err(ParseAttrsError::NoErrType)
    }
}

struct FieldAttrs {
    map: HashMap<Ident, Option<MapType>>,
    with: Option<MapType>,
}
impl FieldAttrs {
    fn check(&self, container_attrs: &ContainerAttrs) {
        for key in self.map.keys() {
            if !container_attrs.from.contains_key(key) {
                panic!("There is no such 'try_from' key as {:?}", key);
            }
        }
    }
    fn map_for(&self, key: &Ident) -> Map {
        match (self.map.get(&key), &self.with) {
            (Some(Some(map)), _) => Map::Lit(map),
            (Some(None), _) => Map::Defualt,
            (None, Some(with)) => Map::With(with),
            (None, None) => Map::TryInto,
        }
    }
}

type MapType = Expr;

enum Map<'a> {
    Lit(&'a MapType),
    With(&'a MapType),
    TryInto,
    Defualt,
}

fn parse_field_attrs(attrs: &[Attribute]) -> Result<FieldAttrs, ParseAttrsError> {
    let mut map = HashMap::new();
    let mut with = None;
    let iter = attrs
        .into_iter()
        .filter(|attr| path_eq(&attr.path, "try_from"));

    for attr in iter {
        let meta = attr.parse_meta().map_err(|_| ParseAttrsError::ParseMeta)?;
        match meta {
            Meta::List(list) => {
                for nested in list.nested {
                    match nested {
                        NestedMeta::Meta(nested_meta) => {
                            match nested_meta {
                                Meta::List(list) => {
                                    let value = single_ident_from_meta_list(&list)?;
                                    match list.path.get_ident().cloned() {
                                        Some(key) if value == "default" => {
                                            if let Some(_old_value) = map.insert(key, None) {
                                                return Err(ParseAttrsError::DuplicateAttributes);
                                            }
                                        }
                                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                                    }
                                }
                                Meta::NameValue(name_value) if path_eq(&name_value.path, "with") => {
                                    let expr: Expr = name_value_parse(&name_value).ok_or(ParseAttrsError::UnsupportedExpressionLiteral)?;
                                    if let Some(_old_with) = with.replace(expr) {
                                        return Err(ParseAttrsError::DuplicateAttributes);
                                    }
                                }
                                Meta::NameValue(name_value) => {
                                    let expr: Expr = name_value_parse(&name_value).ok_or(ParseAttrsError::UnsupportedExpressionLiteral)?;
                                    if let Some(key) = name_value.path.get_ident().cloned() {
                                        if let Some(_old_value) = map.insert(key, Some(expr)) {
                                            return Err(ParseAttrsError::DuplicateAttributes);
                                        }
                                    } else {
                                        return Err(ParseAttrsError::UnsupportedStructure);
                                    }
                                }
                                _ => return Err(ParseAttrsError::UnsupportedStructure),
                            }
                        }
                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                    }
                }
            }
            _ => return Err(ParseAttrsError::UnsupportedStructure),
        }
    }
    Ok(FieldAttrs{map, with})
}

#[derive(Debug)]
enum ParseAttrsError {
    UnsupportedStructure,
    ParseMeta,
    NoPaths,
    DuplicateAttributes,
    NoErrType,
    UnsupportedErrLiteral,
    UnsupportedKeyLiteral,
    UnsupportedExpressionLiteral,
}

/*
enum ExprFunc {
    ExprClosure(ExprClosure),
    ExprPath(ExprPath),
}
impl Parse for ExprFunc {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if let Ok(path)  = input.parse() {
            Ok(Self::ExprPath(path))
        } else {
            Ok(Self::ExprClosure(input.parse()?))
        }
    }
}
*/

fn path_eq(path: &Path, str: &str) -> bool {
    path.get_ident().map(|ident| ident == str).unwrap_or_default()
}

fn single_path_from_meta_list(meta_list: &MetaList) -> Result<&Path, ParseAttrsError> {
    if meta_list.nested.len() != 1 {
        return Err(ParseAttrsError::UnsupportedStructure);
    }
    let nested_meta = meta_list.nested.iter().next().unwrap();
    match nested_meta {
        NestedMeta::Meta(Meta::Path(path)) => Ok(path),
        _ => Err(ParseAttrsError::UnsupportedStructure),
    }
}

fn single_ident_from_meta_list(meta_list: &MetaList) -> Result<&Ident, ParseAttrsError> {
    single_path_from_meta_list(meta_list)?.get_ident().ok_or(ParseAttrsError::UnsupportedStructure)
}

fn name_value_parse<T: Parse>(name_value: &MetaNameValue) -> Option<T> {
    if let Lit::Str(lit_str) = &name_value.lit {
        lit_str.parse().ok()
    } else {
        None
    }
}