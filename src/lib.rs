use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, Attribute, Data, DeriveInput, Expr, Lit,
    Meta, MetaList, MetaNameValue, NestedMeta, Path, Type,
};

mod convert_enum;
mod convert_struct;

#[proc_macro_derive(
    Convert,
    attributes(from, from_self, try_from, try_from_self)
)]
pub fn derive_convert(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    if !input.generics.params.is_empty() {
        unimplemented!("Generics are not supported!");
    }
    let container_attrs = parse_container_attrs(&input.attrs)
        .expect("Parse attributes to find paths from `try_from`");

    match &input.data {
        Data::Struct(data) => convert_struct::derive_convert_struct(
            &container_attrs,
            &input.ident,
            data,
        ),
        Data::Enum(data) => convert_enum::derive_convert_enum(
            &container_attrs,
            &input.ident,
            data,
        ),
        Data::Union(_) => unimplemented!("Unions are not supported!"),
    }
    .into()
}

struct FieldNamer<'a> {
    from_self: bool,
    name: &'a Ident,
    foreign_field: Option<&'a Ident>,
    from: &'a Type,
    to: &'a Type,
}
impl<'a> FieldNamer<'a> {
    fn with<I: Into<Option<&'a Ident>>>(
        &mut self,
        rename: I,
    ) -> (&'a Ident, &'a Ident) {
        if self.from_self {
            (rename.into().unwrap_or(self.name), self.name)
        } else {
            let rename = rename.into();
            self.foreign_field = Some(rename.unwrap_or(self.name));
            (self.name, rename.unwrap_or(self.name))
        }
    }
}

struct ContainerAttrs {
    from: Option<FromAttrs>,
    from_self: Option<FromAttrs>,
    try_from: Option<TryFromAttrs>,
    try_from_self: Option<TryFromAttrs>,
}
impl ContainerAttrs {
    fn is_empty(&self) -> bool {
        self.from.is_none()
            && self.from_self.is_none()
            && self.try_from.is_none()
            && self.try_from_self.is_none()
    }
}

struct TryFromAttrs {
    types: Types,
    err_ty: Type,
}

struct FromAttrs {
    types: Types,
}

struct Types(HashMap<Ident, AttrType>);

struct AttrType {
    ty: Type,
    ignores: Vec<Ident>,
}

impl Types {
    fn iter_with<'a>(
        &'a self,
        subject: &'a Type,
        from_self: bool,
    ) -> impl Iterator<Item = TypeRef<'a>> {
        self.0.iter().map(move |(key, object)| {
            let (from, to) = if from_self {
                (subject, &object.ty)
            } else {
                (&object.ty, subject)
            };
            TypeRef {
                key,
                from,
                to,
                ignores: &object.ignores,
            }
        })
    }
}

#[derive(Clone, Copy)]
struct TypeRef<'a> {
    key: &'a Ident,
    from: &'a Type,
    to: &'a Type,
    ignores: &'a [Ident],
}

fn parse_container_attrs(
    attrs: &[Attribute],
) -> Result<ContainerAttrs, ParseAttrsError> {
    let attrs = ContainerAttrs {
        from: parse_try_from_attrs(attrs, "from")?,
        from_self: parse_try_from_attrs(attrs, "from_self")?,
        try_from: parse_try_from_attrs(attrs, "try_from")?,
        try_from_self: parse_try_from_attrs(attrs, "try_from_self")?,
    };
    if attrs.is_empty() {
        Err(ParseAttrsError::NothingToImplement)
    } else {
        Ok(attrs)
    }
}

struct MaybeFromAttrs {
    types: Types,
    err_ty: Option<Type>,
}

fn parse_try_from_attrs<T>(
    attrs: &[Attribute],
    filter_path: &str,
) -> Result<Option<T>, ParseAttrsError>
where
    MaybeFromAttrs: TryInto<T, Error = ParseAttrsError>,
{
    let mut types = HashMap::new();
    let mut err_ty = None;
    let iter = attrs
        .iter()
        .filter(|attr| path_eq_convert(&attr.path, filter_path));

    let mut attr_is_missing = true;

    for attr in iter {
        attr_is_missing = false;
        let meta = attr.parse_meta().map_err(|_| ParseAttrsError::ParseMeta)?;
        match meta {
            Meta::List(list) => {
                for nested in list.nested {
                    match nested {
                        NestedMeta::Meta(nested_meta) => match nested_meta {
                            Meta::NameValue(name_value)
                                if path_eq(&name_value.path, "Error") =>
                            {
                                let err: Type = lit_parse(&name_value.lit)
                                    .ok_or(
                                        ParseAttrsError::UnsupportedErrLiteral,
                                    )?;
                                if let Some(_old_err) = err_ty.replace(err) {
                                    return Err(
                                        ParseAttrsError::DuplicateAttributes,
                                    );
                                }
                            }
                            Meta::NameValue(name_value) => {
                                let key = name_value
                                    .path
                                    .get_ident()
                                    .cloned()
                                    .ok_or(
                                        ParseAttrsError::UnsupportedStructure,
                                    )?;
                                let map: Type = lit_parse(&name_value.lit)
                                    .ok_or(
                                        ParseAttrsError::UnsupportedKeyLiteral,
                                    )?;
                                if let Some(_old_value) = types.insert(
                                    key,
                                    AttrType {
                                        ty: map,
                                        ignores: vec![],
                                    },
                                ) {
                                    return Err(
                                        ParseAttrsError::DuplicateAttributes,
                                    );
                                }
                            }
                            Meta::List(list) => {
                                let key =
                                    list.path.get_ident().cloned().ok_or(
                                        ParseAttrsError::UnsupportedStructure,
                                    )?;
                                let mut map: Option<Type> = None;
                                let mut ignores = vec![];
                                for meta in list.nested {
                                    match meta {
                                        NestedMeta::Lit(lit) => {
                                            if map.replace(lit_parse(&lit)
                                            .ok_or(ParseAttrsError::UnsupportedKeyLiteral)?).is_some() {
                                                return Err(ParseAttrsError::DuplicateAttributes);
                                            }
                                        }
                                        NestedMeta::Meta(meta) => {
                                            match meta {
                                                Meta::List(list) if path_eq(&list.path, "ignore") => {
                                                    for nested in list.nested {
                                                        match nested {
                                                            NestedMeta::Lit(lit) => {
                                                                let field: Ident = lit_parse(&lit)
                                                                    .ok_or(ParseAttrsError::UnsupportedKeyLiteral)?;
                                                                ignores.push(field);
                                                            }
                                                            _ => return Err(ParseAttrsError::UnsupportedStructure),
                                                        }
                                                    }
                                                }
                                                _ => return Err(ParseAttrsError::UnsupportedStructure),
                                            }
                                        }
                                    }
                                }
                                match map {
                                    None => {
                                        return Err(ParseAttrsError::UnsupportedStructure);
                                    }
                                    Some(map) => {
                                        if let Some(_old_value) = types.insert(
                                            key,
                                            AttrType { ty: map, ignores },
                                        ) {
                                            return Err(ParseAttrsError::DuplicateAttributes);
                                        }
                                    }
                                }
                            }
                            _ => {
                                return Err(
                                    ParseAttrsError::UnsupportedStructure,
                                );
                            }
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
    Ok(Some(
        MaybeFromAttrs {
            err_ty,
            types: Types(types),
        }
        .try_into()?,
    ))
}

impl TryFrom<MaybeFromAttrs> for TryFromAttrs {
    type Error = ParseAttrsError;

    fn try_from(
        MaybeFromAttrs { err_ty, types }: MaybeFromAttrs,
    ) -> Result<Self, Self::Error> {
        if let Some(err_ty) = err_ty {
            Ok(Self { types, err_ty })
        } else {
            Err(ParseAttrsError::NoErrType)
        }
    }
}
impl TryFrom<MaybeFromAttrs> for FromAttrs {
    type Error = ParseAttrsError;

    fn try_from(
        MaybeFromAttrs { err_ty, types }: MaybeFromAttrs,
    ) -> Result<Self, Self::Error> {
        if let Some(_err_ty) = err_ty {
            Err(ParseAttrsError::UnnecessaryErrType)
        } else {
            Ok(Self { types })
        }
    }
}

struct FieldAttrs<FO> {
    map: HashMap<Ident, FO>,
    with: FO,
}
impl<FO> FieldAttrs<FO> {
    fn check(&self, types: &Types) {
        for key in self.map.keys() {
            if !types.0.contains_key(key) {
                panic!("There is no such 'try_from' key as {:?}", key);
            }
        }
    }

    fn map_for(&self, key: &Ident) -> &FO {
        if let Some(map) = self.map.get(key) {
            map
        } else {
            &self.with
        }
    }
}

type MapType = Expr;

enum MapRef {
    Owned,
    Ref,
    Mut,
}

impl ToTokens for MapRef {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(match self {
            MapRef::Owned => return,
            MapRef::Ref => quote!(&),
            MapRef::Mut => quote!(&mut),
        });
    }
}

trait FieldOp: Sized + Default {
    fn rename(self, rename_to: Option<Ident>) -> Result<Self, ParseAttrsError>;

    fn from_key_expr(key: &str, expr: Expr) -> Result<Self, ParseAttrsError>;

    fn from_key(key: &str) -> Result<Self, ParseAttrsError>;

    fn quote<'a>(&'a self, namer: &mut FieldNamer<'a>) -> TokenStream2;
}

fn parse_field_attrs<FO: FieldOp>(
    attrs: &[Attribute],
    filter_path: &str,
) -> Result<FieldAttrs<FO>, ParseAttrsError> {
    let mut map = HashMap::new();
    let mut with = None::<FO>;
    let mut with_rename = None;
    let iter = attrs
        .iter()
        .filter(|attr| path_eq_convert(&attr.path, filter_path));

    for attr in iter {
        let meta = attr.parse_meta().map_err(|_| ParseAttrsError::ParseMeta)?;
        match meta {
            Meta::List(list) => {
                for nested in list.nested {
                    match nested {
                        NestedMeta::Meta(nested_meta) => {
                            match nested_meta {
                                Meta::NameValue(name_value) => {
                                    if let Some(_old_with) = with.replace(
                                        map_from_name_value(&name_value)?,
                                    ) {
                                        return Err(ParseAttrsError::DuplicateAttributes);
                                    }
                                }
                                Meta::Path(path) => {
                                    if let Some(_old_with) =
                                        with.replace(map_from_path(&path)?)
                                    {
                                        return Err(ParseAttrsError::DuplicateAttributes);
                                    }
                                }
                                Meta::List(list) => {
                                    if let Some(key) =
                                        list.path.get_ident().cloned()
                                    {
                                        if &key == "rename" {
                                            let ident =
                                                single_ident_from_meta_list(
                                                    &list,
                                                )?;
                                            if let Some(_old_rename) =
                                                with_rename
                                                    .replace(ident.clone())
                                            {
                                                return Err(ParseAttrsError::DuplicateAttributes);
                                            }
                                        } else {
                                            let with =
                                                map_from_meta_list(&list)?;
                                            if let Some(_old_value) =
                                                map.insert(key, with)
                                            {
                                                return Err(ParseAttrsError::DuplicateAttributes);
                                            }
                                        }
                                    } else {
                                        return Err(ParseAttrsError::UnsupportedStructure);
                                    }
                                }
                            }
                        }
                        _ => return Err(ParseAttrsError::UnsupportedStructure),
                    }
                }
            }
            _ => return Err(ParseAttrsError::UnsupportedStructure),
        }
    }
    Ok(FieldAttrs {
        map,
        with: with.unwrap_or_default().rename(with_rename)?,
    })
}

#[derive(Debug)]
enum ParseAttrsError {
    UnsupportedStructure,
    ParseMeta,
    NoPaths,
    DuplicateAttributes,
    NoErrType,
    UnnecessaryErrType,
    UnsupportedErrLiteral,
    UnsupportedKeyLiteral,
    UnsupportedExpressionLiteral,
    NothingToImplement,
    UnsupportedNameValue,
    UnsupportedPath,
    CantRename,
}

fn map_from_meta_list<FO: FieldOp>(
    meta_list: &MetaList,
) -> Result<FO, ParseAttrsError> {
    let mut map = None::<FO>;
    let mut rename = None;
    for nested in &meta_list.nested {
        match nested {
            NestedMeta::Meta(meta) => match kv_from_meta(meta)? {
                KeyValue::Map(value) => {
                    if let Some(_old_value) = map.replace(value) {
                        return Err(ParseAttrsError::UnsupportedStructure);
                    }
                }
                KeyValue::Rename(value) => {
                    if let Some(_old_value) = rename.replace(value) {
                        return Err(ParseAttrsError::DuplicateAttributes);
                    }
                }
            },
            _ => return Err(ParseAttrsError::UnsupportedStructure),
        }
    }
    map.unwrap_or_default().rename(rename)
}

enum KeyValue<FO> {
    Rename(Ident),
    Map(FO),
}

fn map_from_name_value<FO: FieldOp>(
    name_value: &MetaNameValue,
) -> Result<FO, ParseAttrsError> {
    let ident = name_value
        .path
        .get_ident()
        .ok_or(ParseAttrsError::UnsupportedStructure)?;
    let expr: Expr = lit_parse(&name_value.lit)
        .ok_or(ParseAttrsError::UnsupportedExpressionLiteral)?;
    let key = ident.to_string();
    FO::from_key_expr(&key, expr)
}

fn map_from_path<FO: FieldOp>(path: &Path) -> Result<FO, ParseAttrsError> {
    let ident = path
        .get_ident()
        .ok_or(ParseAttrsError::UnsupportedStructure)?;
    let key = ident.to_string();
    FO::from_key(&key)
}

fn kv_from_meta<FO: FieldOp>(
    meta: &Meta,
) -> Result<KeyValue<FO>, ParseAttrsError> {
    Ok(KeyValue::Map(match meta {
        Meta::NameValue(name_value) => map_from_name_value(name_value)?,
        Meta::Path(path) => map_from_path(path)?,
        Meta::List(list) => {
            if let Some(key) = list.path.get_ident().cloned() {
                if &key == "rename" {
                    let ident = single_ident_from_meta_list(list)?;
                    return Ok(KeyValue::Rename(ident.clone()));
                }
            }
            return Err(ParseAttrsError::UnsupportedStructure);
        }
    }))
}

fn ident_from_meta(meta: &Meta) -> Result<&Ident, ParseAttrsError> {
    match meta {
        Meta::Path(path) => path
            .get_ident()
            .ok_or(ParseAttrsError::UnsupportedStructure),
        _ => Err(ParseAttrsError::UnsupportedStructure),
    }
}

fn path_eq(path: &Path, str: &str) -> bool {
    path.get_ident()
        .map(|ident| ident == str)
        .unwrap_or_default()
}

fn path_eq_convert(path: &Path, str: &str) -> bool {
    path_eq(path, str)
    /*
    let mut iter = path.segments.iter();
    matches!(
        iter.next(),
        Some(convert) if convert.arguments.is_none() && &convert.ident == "convert"
    )
    && matches!(
        (iter.next(), iter.next()),
        (Some(segment), None) if segment.arguments.is_none() && &segment.ident == str
    )
    */
}

fn _single_meta_from_meta_list(
    meta_list: &MetaList,
) -> Result<&Meta, ParseAttrsError> {
    if meta_list.nested.len() != 1 {
        return Err(ParseAttrsError::UnsupportedStructure);
    }
    let nested_meta = meta_list.nested.iter().next().unwrap();
    match nested_meta {
        NestedMeta::Meta(nested_meta) => Ok(nested_meta),
        _ => Err(ParseAttrsError::UnsupportedStructure),
    }
}

fn single_ident_from_meta_list(
    meta_list: &MetaList,
) -> Result<Ident, ParseAttrsError> {
    if meta_list.nested.len() != 1 {
        return Err(ParseAttrsError::UnsupportedStructure);
    }
    let nested_meta = meta_list.nested.iter().next().unwrap();
    Ok(match nested_meta {
        NestedMeta::Meta(nested_meta) => ident_from_meta(nested_meta)?.clone(),
        NestedMeta::Lit(lit) => {
            lit_parse(lit).ok_or(ParseAttrsError::UnsupportedStructure)?
        }
    })
}

fn lit_parse<T: Parse>(lit: &Lit) -> Option<T> {
    if let Lit::Str(lit_str) = lit {
        lit_str.parse().ok()
    } else {
        None
    }
}
