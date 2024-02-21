use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::{DataStruct, Type, TypePath};

use crate::ContainerAttrs;

mod from;
mod try_from;

pub(super) fn derive_convert_struct(
    ContainerAttrs {
        from,
        from_self,
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
        from.as_ref().map(|attrs| {
            from::derive_from_struct(attrs, &subject, data, false)
        }),
        from_self
            .as_ref()
            .map(|attrs| from::derive_from_struct(attrs, &subject, data, true)),
        try_from.as_ref().map(|attrs| {
            try_from::derive_try_from_struct(attrs, &subject, data, false)
        }),
        try_from_self.as_ref().map(|attrs| {
            try_from::derive_try_from_struct(attrs, &subject, data, true)
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}
