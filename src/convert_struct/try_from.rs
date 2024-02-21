use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{DataStruct, Expr, Type};

use super::AllFieldsOptions;
use crate::{
    FieldNamer, FieldOp, MapRef, MapType, ParseAttrsError, TryFromAttrs,
    TypeRef,
};

enum TryFromFieldOp {
    Map {
        expr: MapType,
        rename: Option<Ident>,
        map_ref: MapRef,
    },
    TryMap {
        expr: MapType,
        rename: Option<Ident>,
        map_ref: MapRef,
    },
    New(MapType),
    TryInto {
        rename: Option<Ident>,
    },
    Default,
    Skip,
}

impl Default for TryFromFieldOp {
    fn default() -> Self {
        Self::TryInto { rename: None }
    }
}

impl FieldOp for TryFromFieldOp {
    fn rename(
        mut self,
        rename_to: Option<Ident>,
    ) -> Result<Self, ParseAttrsError> {
        match &mut self {
            Self::Map { rename, .. }
            | Self::TryMap { rename, .. }
            | Self::TryInto { rename } => {
                *rename = rename_to;
                Ok(self)
            }
            _ if rename_to.is_none() => Ok(self),
            _ => Err(ParseAttrsError::CantRename),
        }
    }

    fn from_key_expr(key: &str, expr: Expr) -> Result<Self, ParseAttrsError> {
        Ok(match key {
            "map" => Self::Map {
                expr,
                rename: None,
                map_ref: MapRef::Owned,
            },
            "map_ref" => Self::Map {
                expr,
                rename: None,
                map_ref: MapRef::Ref,
            },
            "map_mut" => Self::Map {
                expr,
                rename: None,
                map_ref: MapRef::Mut,
            },
            "try_map" => Self::TryMap {
                expr,
                rename: None,
                map_ref: MapRef::Owned,
            },
            "try_map_ref" => Self::TryMap {
                expr,
                rename: None,
                map_ref: MapRef::Ref,
            },
            "try_map_mut" => Self::TryMap {
                expr,
                rename: None,
                map_ref: MapRef::Mut,
            },
            "new" => Self::New(expr),
            _ => return Err(ParseAttrsError::UnsupportedNameValue),
        })
    }

    fn from_key(key: &str) -> Result<Self, ParseAttrsError> {
        Ok(match key {
            "default" => Self::Default,
            "skip" => Self::Skip,
            _ => return Err(ParseAttrsError::UnsupportedPath),
        })
    }

    fn quote<'a>(&'a self, namer: &mut FieldNamer<'a>) -> TokenStream2 {
        let name = namer.name;
        match self {
            TryFromFieldOp::Map {
                expr,
                rename,
                map_ref,
            } => {
                let (this, other) = namer.with(rename);
                quote!(#this: (#expr)(#map_ref value.#other),)
            }
            TryFromFieldOp::TryMap {
                expr,
                rename,
                map_ref,
            } => {
                let (this, other) = namer.with(rename);
                quote!(#this: (#expr)(#map_ref value.#other)?,)
            }
            TryFromFieldOp::New(expr) => {
                quote!(#name: (#expr)(),)
            }
            TryFromFieldOp::TryInto { rename } => {
                let (this, other) = namer.with(rename);
                quote!(#this: value.#other.try_into()?,)
            }
            TryFromFieldOp::Default => {
                quote!(#name: Default::default(),)
            }
            TryFromFieldOp::Skip => {
                let _ = namer.with(None);
                quote!()
            }
        }
    }
}

pub(super) fn derive_try_from_struct(
    container_attrs: &TryFromAttrs,
    subject: &Type,
    data: &DataStruct,
    from_self: bool,
) -> TokenStream2 {
    let filter_path = if from_self {
        "try_from_self"
    } else {
        "try_from"
    };
    let TryFromAttrs { types, err_ty } = container_attrs;
    let fields = AllFieldsOptions::<TryFromFieldOp>::parse(
        &data.fields,
        filter_path,
        types,
    );

    types
        .iter_with(subject, from_self)
        .map(|type_ref @ TypeRef { from, to, .. }| {
            let (lines, foreign_fields) =
                fields.lines_n_fields(from_self, type_ref);
            quote! {
                impl std::convert::TryFrom<#from> for #to {
                    type Error = #err_ty;

                    fn try_from(value: #from) -> Result<#to, Self::Error> {
                        #foreign_fields
                        Ok(Self {
                            #lines
                        })
                    }
                }
            }
        })
        .collect()
}
