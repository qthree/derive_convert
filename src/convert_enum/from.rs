use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DataEnum, Expr, Ident, Type};

use super::AllVariantsOptions;
use crate::{FieldNamer, FieldOp, FromAttrs, ParseAttrsError, TypeRef};

enum FromVariantOp {
    Into { rename: Option<Ident> },
    Default,
    Skip,
}

impl Default for FromVariantOp {
    fn default() -> Self {
        Self::Into { rename: None }
    }
}

impl FieldOp for FromVariantOp {
    fn rename(
        mut self,
        rename_to: Option<Ident>,
    ) -> Result<Self, ParseAttrsError> {
        match &mut self {
            Self::Into { rename } => {
                *rename = rename_to;
                Ok(self)
            }
            _ if rename_to.is_none() => Ok(self),
            _ => Err(ParseAttrsError::CantRename),
        }
    }

    fn from_key_expr(_key: &str, _expr: Expr) -> Result<Self, ParseAttrsError> {
        Err(ParseAttrsError::UnsupportedNameValue)
    }

    fn from_key(key: &str) -> Result<Self, ParseAttrsError> {
        Ok(match key {
            "default" => Self::Default,
            "skip" => Self::Skip,
            _ => return Err(ParseAttrsError::UnsupportedPath),
        })
    }

    fn quote<'a>(
        &'a self,
        namer @ &mut FieldNamer { from, to, .. }: &mut FieldNamer<'a>,
    ) -> TokenStream2 {
        let name = namer.name;
        match self {
            FromVariantOp::Into { rename } => {
                let (this, other) = namer.with(rename);
                quote!(#from::#other => #to::#this,)
            }
            FromVariantOp::Default => {
                let _ = namer.with(None);
                quote!(#from::#name => #to::default(),)
            }
            FromVariantOp::Skip => {
                quote!()
            }
        }
    }
}

pub(super) fn derive_from_enum(
    FromAttrs { types }: &FromAttrs,
    subject: &Type,
    data: &DataEnum,
    from_self: bool,
) -> TokenStream2 {
    let filter_path = if from_self { "from_self" } else { "from" };
    let variants =
        AllVariantsOptions::<FromVariantOp>::parse(data, filter_path, types);
    types
        .iter_with(subject, from_self)
        .map(|type_ref @ TypeRef { from, to, .. }| {
            let (lines, foreign_fields) =
                variants.lines_n_fields(from_self, type_ref);
            quote! {
                impl std::convert::From<#from> for #to {
                    fn from(value: #from) -> #to {
                        #foreign_fields
                        match value {
                            #lines
                        }
                    }
                }
            }
        })
        .collect()
}
