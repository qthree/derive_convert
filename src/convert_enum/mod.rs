mod from;
mod try_from;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{DataEnum, Type, TypePath};

use crate::{
    parse_field_attrs, ContainerAttrs, FieldAttrs, FieldNamer, FieldOp,
    TypeRef, Types,
};

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

struct OneVariantOptions<'a, FO> {
    name: &'a Ident,
    attrs: FieldAttrs<FO>,
}

struct AllVariantsOptions<'a, FO> {
    variants: Vec<OneVariantOptions<'a, FO>>,
}

impl<'a, FO: FieldOp> AllVariantsOptions<'a, FO> {
    fn parse(data: &'a DataEnum, filter_path: &str, types: &Types) -> Self {
        let variants = &data.variants;
        let variants = variants
            .into_iter()
            .map(|variant| {
                if !variant.fields.is_empty() {
                    unimplemented!("Only C-like enums are supported");
                }
                let attrs = parse_field_attrs(&variant.attrs, filter_path)
                    .expect("Parse attributes to find field options");
                attrs.check(types);
                OneVariantOptions {
                    name: &variant.ident,
                    attrs,
                }
            })
            .collect();
        Self { variants }
    }

    fn lines_n_fields(
        &self,
        from_self: bool,
        TypeRef {
            key,
            from,
            to,
            ignores,
        }: TypeRef,
    ) -> (TokenStream2, TokenStream2) {
        let mut foreign_fields = ignores.to_owned();
        let lines = self.variants.iter().map(|field| {
            let name = field.name;
            let mut namer = FieldNamer {
                from_self,
                name,
                foreign_field: None,
                from,
                to,
            };
            let res = field.attrs.map_for(key).quote(&mut namer);
            foreign_fields.extend(namer.foreign_field.into_iter().cloned());
            res
        });
        let lines = quote!(
            #(
                #lines
            )*
        );
        let foreign_fields = quote_foreign_fields(from, &foreign_fields);
        (lines, foreign_fields)
    }
}

fn quote_foreign_fields(from: &Type, foreign_fields: &[Ident]) -> TokenStream2 {
    if foreign_fields.is_empty() {
        quote!()
    } else {
        quote!(
            {
                match &value {
                    #(
                        #from::#foreign_fields => {},
                    )*
                }
            }
        )
    }
}
