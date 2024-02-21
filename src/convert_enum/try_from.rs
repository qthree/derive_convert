use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DataEnum, Type};

use super::variants_from_data_enum;
use crate::{TryFromAttrs, TypeRef};

pub(super) fn derive_try_from_enum(
    TryFromAttrs { types, err_ty }: &TryFromAttrs,
    subject: &Type,
    data: &DataEnum,
    from_self: bool,
) -> TokenStream2 {
    let variants = variants_from_data_enum(data);
    types
        .iter_with(subject, from_self)
        .map(|TypeRef { from, to, .. }| {
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
