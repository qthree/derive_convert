use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{DataStruct, Fields, Member, Type, TypePath};

use crate::{
    ContainerAttrs, FieldAttrs, FieldNamer, FieldOp, TypeRef, Types,
    parse_field_attrs,
};

pub(super) mod from;
pub(super) mod try_from;

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

struct OneFieldOptions<FO> {
    name: Member,
    attrs: FieldAttrs<FO>,
}

pub(crate) struct AllFieldsOptions<FO> {
    fields: Vec<OneFieldOptions<FO>>,
}

enum ForeignField {
    NextPos(usize),
    Name(Member),
}

impl<FO: FieldOp<FieldContext = (), Member = Member>> AllFieldsOptions<FO> {
    pub(crate) fn parse(fields: &Fields, filter_path: &str, types: &Types) -> Self {
        let fields = match fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(fields) => &fields.unnamed,
            _ => unimplemented!("Unit structs are unsupported"),
        };
        let fields = fields
            .into_iter()
            .enumerate()
            .map(|(i, field)| {
                let attrs = parse_field_attrs(&field.attrs, filter_path)
                    .expect("Parse attributes to find field options");
                attrs.check(types);
                OneFieldOptions {
                    name: field.ident.clone().map_or_else(|| Member::Unnamed(i.into()), Member::Named),
                    attrs,
                }
            })
            .collect();
        AllFieldsOptions { fields }
    }

    pub(crate) fn len(&self) -> usize {
        self.fields.len()
    }

    fn lines_n_fields(
        &self,
        from_self: bool,
        TypeRef {
            key,
            from,
            to,
            extra_foreign_fields,
        }: TypeRef,
    ) -> (TokenStream2, TokenStream2) {
        let mut foreign_fields: Vec<_> =
            extra_foreign_fields.ignores().cloned().map(Member::from).collect();
        let lines = self.fields.iter().map(|field| {
            let name = &field.name;
            let mut namer = FieldNamer {
                from_self,
                name,
                name_slot: ,
                from,
                to,
                context: &(),
            };
            let res = field.attrs.map_for(key).quote(&mut namer);
            foreign_fields.extend(namer.name_slot.into_iter().cloned());
            res
        });
        let extra_lines = extra_foreign_fields
            .defaults()
            .map(|name| quote!(#name: Default::default(),));
        let lines = quote!(
            #(
                #lines
            )*
            #(
                #extra_lines
            )*
        );
        let foreign_fields = quote_foreign_fields(from, &foreign_fields);
        (lines, foreign_fields)
    }
}

fn quote_foreign_fields(from: &Type, foreign_fields: &[Member]) -> TokenStream2 {
    if foreign_fields.is_empty() {
        quote!()
    } else {
        quote!(
            {
                if let #from { #(
                    #foreign_fields: _,
                )* } = &value;
            }
        )
    }
}
