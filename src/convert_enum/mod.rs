mod from;
mod try_from;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::{DataEnum, Type, TypePath};

use crate::ContainerAttrs;

pub(super) fn derive_convert_enum(
    ContainerAttrs {
        from,
        from_self,
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
        from.as_ref()
            .map(|attrs| from::derive_from_enum(attrs, &subject, data, false)),
        from_self
            .as_ref()
            .map(|attrs| from::derive_from_enum(attrs, &subject, data, true)),
        try_from.as_ref().map(|attrs| {
            try_from::derive_try_from_enum(attrs, &subject, data, false)
        }),
        try_from_self.as_ref().map(|attrs| {
            try_from::derive_try_from_enum(attrs, &subject, data, true)
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn variants_from_data_enum(data: &DataEnum) -> Vec<&Ident> {
    data.variants
        .iter()
        .map(|var| {
            if !var.fields.is_empty() {
                unimplemented!("Only C-like enums are supported")
            }
            &var.ident
        })
        .collect()
}
