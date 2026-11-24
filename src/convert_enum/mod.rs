mod from;
mod try_from;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{DataEnum, Fields, FieldsNamed, FieldsUnnamed, Member, Type, TypePath, spanned::Spanned};

use crate::{
    ContainerAttrs, FieldAttrs, FieldNamer, FieldOp, TypeRef, Types, convert_struct::AllFieldsOptions, parse_field_attrs,
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

struct OneVariantOptions<'a, FO: FieldOp> {
    name: &'a Ident,
    context: FO::FieldContext,
    attrs: FieldAttrs<FO>,
}

struct VariantContext<FieldsOps> {
    fields: Option<AllFieldsOptions<FieldsOps>>,
}

struct VariantField {
    from: Member,
    to: Member,
}

struct AllVariantsOptions<'a, FO: FieldOp> {
    variants: Vec<OneVariantOptions<'a, FO>>,
}

impl<'a, FO: FieldOp<FieldContext = VariantContext<FieldsOps>, Member = Ident>, FieldsOps: FieldOp<FieldContext = (), Member = Member>>
    AllVariantsOptions<'a, FO>
{
    fn parse(data: &'a DataEnum, filter_path: &str, types: &Types) -> Self {
        let variants = &data.variants;
        let variants = variants
            .into_iter()
            .map(|variant| {
                let fields = if variant.fields.is_empty() {
                    None
                } else {
                    let fields = AllFieldsOptions::<FieldsOps>::parse(&variant.fields, filter_path, types);
                    if fields.len() != 1 {
                        unimplemented!("Only C-like enums or single field variants are supported: {:?}", variant.fields);
                    }
                    Some(fields)
                };
                let attrs = parse_field_attrs(&variant.attrs, filter_path)
                    .expect("Parse attributes to find field options");
                attrs.check(types);
                OneVariantOptions {
                    name: &variant.ident,
                    context: VariantContext { fields },
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
            extra_foreign_fields,
        }: TypeRef,
    ) -> (TokenStream2, TokenStream2) {
        let mut foreign_fields: Vec<_> =
            extra_foreign_fields.ignores().cloned().collect();
        let mut slot = 0;
        let lines = self.variants.iter().map(|field| {
            let name = field.name;
            let mut namer = FieldNamer {
                from_self,
                name,
                name_slot: Err(slot),
                from,
                to,
                context: &field.context,
            };
            let res = field.attrs.map_for(key).quote(&mut namer);
            if let Ok(name_slot) = namer.name_slot {
                foreign_fields.push(name_slot.clone());
                slot += 1;
            }
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
                        #from::#foreign_fields{..} => {},
                    )*
                }
            }
        )
    }
}
/*
fn variant_default(namer: &mut FieldNamer<'_, VariantContext>) -> TokenStream2 {
        let name = namer.name;
        let _ = namer.with(None);

        if let Some(_) = has_one_field {
            quote!(#from::#name(_) => #to::default(),)
        } else {
            quote!(#from::#name => #to::default(),)
        }
}
*/