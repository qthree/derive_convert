use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse::Parse, parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr,
    Fields, Lit, Meta, MetaList, MetaNameValue, NestedMeta, Path, Type, TypePath,
};

#[proc_macro_derive(TryFrom, attributes(try_from, try_from_self))]
pub fn derive_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if !input.generics.params.is_empty() {
        unimplemented!("Generics are not supported!");
    }
    let container_attrs = parse_container_attrs(&input.attrs)
        .expect("Parse attributes to find paths from `try_from`");

    match &input.data {
        Data::Struct(data) => derive_convert_struct(&container_attrs, &input.ident, data),
        Data::Enum(data) => derive_convert_enum(&container_attrs, &input.ident, data),
        Data::Union(_) => unimplemented!("Unions are not supported!"),
    }
    .into()
}

fn derive_convert_struct(
    ContainerAttrs {
        try_from,
        try_from_self,
    }: &ContainerAttrs,
    subject: &Ident,
    data: &DataStruct,
) -> TokenStream2 {
    let subject = Type::Path(TypePath {
        qself: None,
        path: subject.clone().into(),
    });
    [
        try_from
            .as_ref()
            .map(|attrs| derive_try_from_struct(attrs, &subject, data, false)),
        try_from_self
            .as_ref()
            .map(|attrs| derive_try_from_struct(attrs, &subject, data, true)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn derive_try_from_struct(
    container_attrs: &TryFromAttrs,
    subject: &Type,
    data: &DataStruct,
    from_self: bool,
) -> TokenStream2 {
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => unimplemented!("Only structs with named fields are supported"),
    };
    struct FieldOptions<'a> {
        name: &'a Ident,
        attrs: FieldAttrs,
    }
    let filter_path = if from_self {
        "try_from_self"
    } else {
        "try_from"
    };
    let fields: Vec<_> = fields
        .into_iter()
        .map(|field| {
            let attrs = parse_field_attrs(&field.attrs, filter_path)
                .expect("Parse attributes to find options for `try_from`");
            attrs.check(container_attrs);
            FieldOptions {
                name: field.ident.as_ref().unwrap(),
                attrs,
            }
        })
        .collect();

    let TryFromAttrs { types, err_ty } = container_attrs;

    types
        .into_iter()
        .map(|(object_key, object)| {
            let (from, to) = if from_self {
                (subject, object)
            } else {
                (object, subject)
            };
            let lines = fields.iter().map(|field| {
                let name = field.name;
                match field.attrs.map_for(object_key) {
                    Map::Map(expr) => {
                        quote!(#name: (#expr)(value.#name),)
                    }
                    Map::TryMap(expr) => {
                        quote!(#name: (#expr)(value.#name)?,)
                    }
                    Map::New(expr) => {
                        quote!(#name: (#expr)(),)
                    }
                    Map::TryInto => {
                        quote!(#name: value.#name.try_into()?,)
                    }
                    Map::Defualt => {
                        quote!(#name: Default::default(),)
                    }
                    Map::Skip => {
                        quote!()
                    }
                }
            });
            quote! {
                impl std::convert::TryFrom<#from> for #to {
                    type Error = #err_ty;

                    fn try_from(value: #from) -> Result<#to, Self::Error> {
                        Ok(Self {
                            #(
                                #lines
                            )*
                        })
                    }
                }
            }
        })
        .collect()
}

fn derive_convert_enum(
    ContainerAttrs {
        try_from,
        try_from_self,
    }: &ContainerAttrs,
    subject: &Ident,
    data: &DataEnum,
) -> TokenStream2 {
    let subject = Type::Path(TypePath {
        qself: None,
        path: subject.clone().into(),
    });
    [
        try_from
            .as_ref()
            .map(|attrs| derive_try_from_enum(attrs, &subject, data, false)),
        try_from_self
            .as_ref()
            .map(|attrs| derive_try_from_enum(attrs, &subject, data, true)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn derive_try_from_enum(
    TryFromAttrs { types, err_ty }: &TryFromAttrs,
    subject: &Type,
    data: &DataEnum,
    from_self: bool,
) -> TokenStream2 {
    let variants: Vec<_> = data
        .variants
        .iter()
        .map(|var| {
            if !var.fields.is_empty() {
                unimplemented!("Only C-like enums are supported")
            }
            &var.ident
        })
        .collect();
    types
        .values()
        .map(|object| {
            let (from, to) = if from_self {
                (subject, object)
            } else {
                (object, subject)
            };
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
        })
        .collect()
}

struct ContainerAttrs {
    try_from: Option<TryFromAttrs>,
    try_from_self: Option<TryFromAttrs>,
}

struct TryFromAttrs {
    types: HashMap<Ident, Type>,
    err_ty: Type,
}

fn parse_container_attrs(attrs: &[Attribute]) -> Result<ContainerAttrs, ParseAttrsError> {
    let attrs = ContainerAttrs {
        try_from: parse_try_from_attrs(attrs, "try_from")?,
        try_from_self: parse_try_from_attrs(attrs, "try_from_self")?,
    };
    if attrs.try_from.is_none() && attrs.try_from_self.is_none() {
        Err(ParseAttrsError::NothingToImplement)
    } else {
        Ok(attrs)
    }
}

fn parse_try_from_attrs(
    attrs: &[Attribute],
    filter_path: &str,
) -> Result<Option<TryFromAttrs>, ParseAttrsError> {
    let mut types = HashMap::new();
    let mut err_ty = None;
    let iter = attrs
        .into_iter()
        .filter(|attr| path_eq(&attr.path, filter_path));

    let mut attr_is_missing = true;

    for attr in iter {
        attr_is_missing = false;
        let meta = attr.parse_meta().map_err(|_| ParseAttrsError::ParseMeta)?;
        match meta {
            Meta::List(list) => {
                for nested in list.nested {
                    match nested {
                        NestedMeta::Meta(nested_meta) => match nested_meta {
                            Meta::NameValue(name_value) if path_eq(&name_value.path, "Error") => {
                                let err: Type = name_value_parse(&name_value)
                                    .ok_or(ParseAttrsError::UnsupportedErrLiteral)?;
                                if let Some(_old_err) = err_ty.replace(err) {
                                    return Err(ParseAttrsError::DuplicateAttributes);
                                }
                            }
                            Meta::NameValue(name_value) => {
                                let key = name_value
                                    .path
                                    .get_ident()
                                    .cloned()
                                    .ok_or(ParseAttrsError::UnsupportedStructure)?;
                                let map: Type = name_value_parse(&name_value)
                                    .ok_or(ParseAttrsError::UnsupportedKeyLiteral)?;
                                if let Some(_old_value) = types.insert(key, map) {
                                    return Err(ParseAttrsError::DuplicateAttributes);
                                }
                            }
                            _ => return Err(ParseAttrsError::UnsupportedStructure),
                        },
                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                    }
                }
            }
            _ => return Err(ParseAttrsError::UnsupportedStructure),
        }
    }
    if attr_is_missing {
        return Ok(None);
    }
    if types.is_empty() {
        return Err(ParseAttrsError::NoPaths);
    }
    if let Some(err_ty) = err_ty {
        Ok(Some(TryFromAttrs { err_ty, types }))
    } else {
        Err(ParseAttrsError::NoErrType)
    }
}

struct FieldAttrs {
    map: HashMap<Ident, Map>,
    with: Map,
}
impl FieldAttrs {
    fn check(&self, container_attrs: &TryFromAttrs) {
        for key in self.map.keys() {
            if !container_attrs.types.contains_key(key) {
                panic!("There is no such 'try_from' key as {:?}", key);
            }
        }
    }
    fn map_for(&self, key: &Ident) -> &Map {
        if let Some(map) = self.map.get(&key) {
            map
        } else {
            &self.with
        }
    }
}

type MapType = Expr;

enum Map {
    Map(MapType),
    TryMap(MapType),
    New(MapType),
    TryInto,
    Defualt,
    Skip,
}

fn parse_field_attrs(
    attrs: &[Attribute],
    filter_path: &str,
) -> Result<FieldAttrs, ParseAttrsError> {
    let mut map = HashMap::new();
    let mut with = None;
    let iter = attrs
        .into_iter()
        .filter(|attr| path_eq(&attr.path, filter_path));

    for attr in iter {
        let meta = attr.parse_meta().map_err(|_| ParseAttrsError::ParseMeta)?;
        match meta {
            Meta::List(list) => {
                for nested in list.nested {
                    match nested {
                        NestedMeta::Meta(nested_meta) => match nested_meta {
                            Meta::NameValue(name_value) => {
                                if let Some(_old_with) =
                                    with.replace(map_from_name_value(&name_value)?)
                                {
                                    return Err(ParseAttrsError::DuplicateAttributes);
                                }
                            }
                            Meta::Path(path) => {
                                if let Some(_old_with) = with.replace(map_from_path(&path)?) {
                                    return Err(ParseAttrsError::DuplicateAttributes);
                                }
                            }
                            Meta::List(list) => {
                                if let Some(key) = list.path.get_ident().cloned() {
                                    let meta = single_meta_from_meta_list(&list)?;
                                    let with = map_from_meta(meta)?;
                                    if let Some(_old_value) = map.insert(key, with) {
                                        return Err(ParseAttrsError::DuplicateAttributes);
                                    }
                                } else {
                                    return Err(ParseAttrsError::UnsupportedStructure);
                                }
                            }
                        },
                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                    }
                }
            }
            _ => return Err(ParseAttrsError::UnsupportedStructure),
        }
    }
    Ok(FieldAttrs {
        map,
        with: with.unwrap_or(Map::TryInto),
    })
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
    NothingToImplement,
    UnsupportedNameValue,
    UnsupportedPath,
}

fn map_from_name_value(name_value: &MetaNameValue) -> Result<Map, ParseAttrsError> {
    let ident = name_value
        .path
        .get_ident()
        .ok_or(ParseAttrsError::UnsupportedStructure)?;
    let expr: Expr =
        name_value_parse(&name_value).ok_or(ParseAttrsError::UnsupportedExpressionLiteral)?;
    Ok(if ident == "map" {
        Map::Map(expr)
    } else if ident == "try_map" {
        Map::TryMap(expr)
    } else if ident == "new" {
        Map::New(expr)
    } else {
        return Err(ParseAttrsError::UnsupportedNameValue);
    })
}

fn map_from_path(path: &Path) -> Result<Map, ParseAttrsError> {
    let ident = path
        .get_ident()
        .ok_or(ParseAttrsError::UnsupportedStructure)?;
    Ok(if ident == "default" {
        Map::Defualt
    } else if ident == "skip" {
        Map::Skip
    } else {
        return Err(ParseAttrsError::UnsupportedPath);
    })
}

fn map_from_meta(meta: &Meta) -> Result<Map, ParseAttrsError> {
    match meta {
        Meta::NameValue(name_value) => map_from_name_value(name_value),
        Meta::Path(path) => map_from_path(path),
        _ => Err(ParseAttrsError::UnsupportedStructure),
    }
}

fn path_eq(path: &Path, str: &str) -> bool {
    path.get_ident()
        .map(|ident| ident == str)
        .unwrap_or_default()
}

fn single_meta_from_meta_list(meta_list: &MetaList) -> Result<&Meta, ParseAttrsError> {
    if meta_list.nested.len() != 1 {
        return Err(ParseAttrsError::UnsupportedStructure);
    }
    let nested_meta = meta_list.nested.iter().next().unwrap();
    match nested_meta {
        NestedMeta::Meta(nested_meta) => Ok(nested_meta),
        _ => Err(ParseAttrsError::UnsupportedStructure),
    }
}

fn name_value_parse<T: Parse>(name_value: &MetaNameValue) -> Option<T> {
    if let Lit::Str(lit_str) = &name_value.lit {
        lit_str.parse().ok()
    } else {
        None
    }
}
