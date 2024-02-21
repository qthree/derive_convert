use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{DataStruct, Expr, Type};

use super::AllFieldsOptions;
use crate::{
    FieldNamer, FieldOp, FromAttrs, MapRef, MapType, ParseAttrsError, TypeRef,
};

enum FromFieldOp {
    Map {
        expr: MapType,
        rename: Option<Ident>,
        map_ref: MapRef,
    },
    New(MapType),
    Into {
        rename: Option<Ident>,
    },
    Default,
    Skip,
}

impl Default for FromFieldOp {
    fn default() -> Self {
        Self::Into { rename: None }
    }
}

impl FieldOp for FromFieldOp {
    fn rename(
        mut self,
        rename_to: Option<Ident>,
    ) -> Result<Self, ParseAttrsError> {
        match &mut self {
            Self::Map { rename, .. } | Self::Into { rename } => {
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
            FromFieldOp::Map {
                expr,
                rename,
                map_ref,
            } => {
                let (this, other) = namer.with(rename);
                quote!(#this: (#expr)(#map_ref value.#other),)
            }
            FromFieldOp::New(expr) => {
                quote!(#name: (#expr)(),)
            }
            FromFieldOp::Into { rename } => {
                let (this, other) = namer.with(rename);
                quote!(#this: value.#other.into(),)
            }
            FromFieldOp::Default => {
                quote!(#name: Default::default(),)
            }
            FromFieldOp::Skip => {
                let _ = namer.with(None);
                quote!()
            }
        }
    }
}

pub(super) fn derive_from_struct(
    container_attrs: &FromAttrs,
    subject: &Type,
    data: &DataStruct,
    from_self: bool,
) -> TokenStream2 {
    let filter_path = if from_self { "from_self" } else { "from" };
    let FromAttrs { types } = container_attrs;
    let fields = AllFieldsOptions::<FromFieldOp>::parse(
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
                impl std::convert::From<#from> for #to {
                    fn from(value: #from) -> #to {
                        #foreign_fields
                        Self {
                            #lines
                        }
                    }
                }
            }
        })
        .collect()
}
