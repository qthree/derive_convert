use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{DataStruct, Fields, Type};

use crate::{
    parse_field_attrs, quote_foreign_fields, FieldAttrs, FieldNamer, FieldOp,
    TryFromAttrs, TypeRef,
};

pub(super) fn derive_try_from_struct(
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
                .expect("Parse attributes to find field options");
            attrs.check(container_attrs);
            FieldOptions {
                name: field.ident.as_ref().unwrap(),
                attrs,
            }
        })
        .collect();

    let TryFromAttrs { types, err_ty } = container_attrs;

    types
        .iter_with(subject, from_self)
        .map(
            |TypeRef {
                 key,
                 from,
                 to,
                 ignores,
             }| {
                let mut foreign_fields = ignores.to_owned();
                let lines = fields.iter().map(|field| {
                    let name = field.name;
                    let mut namer = FieldNamer {
                        from_self,
                        name,
                        foreign_field: None,
                    };
                    let res = match field.attrs.map_for(key) {
                        FieldOp::Map {
                            expr,
                            rename,
                            map_ref,
                        } => {
                            let (this, other) = namer.with(rename);
                            quote!(#this: (#expr)(#map_ref value.#other),)
                        }
                        FieldOp::TryMap {
                            expr,
                            rename,
                            map_ref,
                        } => {
                            let (this, other) = namer.with(rename);
                            quote!(#this: (#expr)(#map_ref value.#other)?,)
                        }
                        FieldOp::New(expr) => {
                            quote!(#name: (#expr)(),)
                        }
                        FieldOp::TryInto { rename } => {
                            let (this, other) = namer.with(rename);
                            quote!(#this: value.#other.try_into()?,)
                        }
                        FieldOp::Defualt => {
                            quote!(#name: Default::default(),)
                        }
                        FieldOp::Skip => {
                            let _ = namer.with(None);
                            quote!()
                        }
                    };
                    foreign_fields
                        .extend(namer.foreign_field.into_iter().cloned());
                    res
                });
                let lines = quote!(
                    #(
                        #lines
                    )*
                );
                let foreign_fields =
                    quote_foreign_fields(from, &foreign_fields);
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
            },
        )
        .collect()
}
