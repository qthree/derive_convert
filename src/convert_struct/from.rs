use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DataStruct, Fields, Type};

use crate::{quote_foreign_fields, FromAttrs, TypeRef};

pub(super) fn derive_from_struct(
    container_attrs: &FromAttrs,
    subject: &Type,
    data: &DataStruct,
    from_self: bool,
) -> TokenStream2 {
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => unimplemented!("Only structs with named fields are supported"),
    };

    let FromAttrs { types } = container_attrs;

    types
        .iter_with(subject, from_self)
        .map(
            |TypeRef {
                 from, to, ignores, ..
             }| {
                let mut foreign_fields = ignores.to_owned();
                let lines = fields.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    foreign_fields.push(name.clone());
                    quote!(#name: value.#name.into(),)
                });
                let lines = quote!(
                    #(
                        #lines
                    )*
                );
                let foreign_fields =
                    quote_foreign_fields(from, &foreign_fields);
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
            },
        )
        .collect()
}
