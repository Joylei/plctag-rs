// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use syn::{Attribute, Data, DataStruct, Fields, Ident, Lit, Meta, NestedMeta};

pub fn get_crate() -> syn::Result<Ident> {
    let plctag = match crate_name("plctag").or_else(|_| crate_name("plctag-core")) {
        Ok(found) => match found {
            FoundCrate::Itself => Ident::new("crate", Span::call_site()),
            FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        },
        Err(_) => Ident::new("crate", Span::call_site()),
    };
    Ok(plctag)
}

pub fn get_fields(data: Data) -> syn::Result<Vec<(Ident, u32)>> {
    let fields = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => fields.named,
        _ => panic!("this derive macro only works on structs with named fields"),
    };
    let items = fields
        .into_iter()
        .map(|f| {
            let attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("offset"))
                .collect();
            assert!(attrs.len() > 0);
            let offset = match attrs.len() {
                0 => return Ok(None),
                1 => get_offset_attr(&attrs[0])?,
                _ => {
                    let mut error =
                        syn::Error::new_spanned(&attrs[1], "redundant `offset()` attribute");
                    error.combine(syn::Error::new_spanned(&attrs[0], "note: first one here"));
                    return Err(error);
                }
            };
            let field_name = f.ident.unwrap();
            Ok(Some((field_name, offset)))
        })
        .filter_map(|res| match res {
            Ok(None) => None,
            Ok(Some(v)) => Some(Ok(v)),
            Err(e) => Some(Err(e)),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if items.len() == 0 {
        panic!("this derive macro requires at least one offset() attribute on structs")
    }
    Ok(items)
}

fn get_offset_attr(attr: &Attribute) -> syn::Result<u32> {
    let meta = attr.parse_meta()?;
    //offset()
    let meta_list = match meta {
        Meta::List(list) => list,
        _ => {
            return Err(syn::Error::new_spanned(
                meta,
                "bad usage, please refer to offset attribute",
            ))
        }
    };

    //extract nested from offset(nested)
    let nested = match meta_list.nested.len() {
        1 => &meta_list.nested[0],
        _ => {
            return Err(syn::Error::new_spanned(
                meta_list.nested,
                "currently only a single offset attribute is supported",
            ));
        }
    };

    let offset_value = match nested {
        NestedMeta::Lit(offset_value) => offset_value,
        _ => {
            return Err(syn::Error::new_spanned(
                meta_list.nested,
                "bad usage, please refer to offset attribute",
            ));
        }
    };

    match &offset_value {
        Lit::Int(s) => s.base10_parse(),
        lit => Err(syn::Error::new_spanned(lit, "expected int literal")),
    }
}
