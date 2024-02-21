use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DataEnum, Type};

use super::variants_from_data_enum;
use crate::{FromAttrs, TypeRef};

pub(super) fn derive_from_enum(
    FromAttrs { types }: &FromAttrs,
    subject: &Type,
    data: &DataEnum,
    from_self: bool,
) -> TokenStream2 {
    let variants = variants_from_data_enum(data);
    types
        .iter_with(subject, from_self)
        .map(|TypeRef { from, to, .. }| {
            quote! {
                impl std::convert::From<#from> for #to {
                    fn from(value: #from) -> #to {
                        match value {
                            #(
                                #from::#variants => #to::#variants,
                            )*
                        }
                    }
                }
            }
        })
        .collect()
}
