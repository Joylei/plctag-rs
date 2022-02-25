// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use syn::{Attribute, Data, DataStruct, Fields, Ident, Lit, Meta, NestedMeta, Type};

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

pub fn get_fields(data: Data) -> syn::Result<Vec<(Ident, Type, TagInfo)>> {
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
                .filter(|attr| attr.path.is_ident("tag"))
                .collect();
            assert!(!attrs.is_empty());
            let offset = match attrs.len() {
                0 => return Ok(None),
                1 => get_tag_attr(attrs[0])?,
                _ => {
                    let mut error =
                        syn::Error::new_spanned(&attrs[1], "redundant `tag()` attribute");
                    error.combine(syn::Error::new_spanned(&attrs[0], "note: first one here"));
                    return Err(error);
                }
            };
            let field_name = f.ident.unwrap();
            let ty = f.ty;
            Ok(Some((field_name, ty, offset)))
        })
        .filter_map(|res| match res {
            Ok(None) => None,
            Ok(Some(v)) => Some(Ok(v)),
            Err(e) => Some(Err(e)),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if items.is_empty() {
        panic!("this derive macro requires at least one tag() attribute on structs")
    }
    Ok(items)
}

fn get_tag_attr(attr: &Attribute) -> syn::Result<TagInfo> {
    let meta = attr.parse_meta()?;
    //tag()
    let meta_list = match meta {
        Meta::List(list) => list,
        _ => {
            return Err(syn::Error::new_spanned(
                meta,
                "bad usage, please refer to tag attribute",
            ))
        }
    };

    //extract nested from tag(nested)
    let nested = match meta_list.nested.len() {
        1 => &meta_list.nested,
        _ => {
            return Err(syn::Error::new_spanned(
                meta_list.nested,
                "currently only a single tag attribute is supported",
            ));
        }
    };

    let mut offset = None;
    let mut size = None;
    for item in nested {
        let name_value = match item {
            NestedMeta::Meta(Meta::NameValue(nv)) => nv,
            _ => {
                return Err(syn::Error::new_spanned(
                    nested,
                    "expected `offset = \"<value>\"` or `size = \"<value>\"`",
                ))
            }
        };

        if name_value.path.is_ident("offset") {
            match &name_value.lit {
                Lit::Int(s) => {
                    if offset.is_some() {
                        return Err(syn::Error::new_spanned(
                            s,
                            "redundant definition for offset",
                        ));
                    }
                    offset = Some(s.base10_parse()?);
                }
                lit => return Err(syn::Error::new_spanned(lit, "expected int literal")),
            }
        } else if name_value.path.is_ident("size") {
            match &name_value.lit {
                Lit::Int(s) => {
                    if size.is_some() {
                        return Err(syn::Error::new_spanned(s, "redundant definition for size"));
                    }
                    size = Some(s.base10_parse()?);
                }
                lit => return Err(syn::Error::new_spanned(lit, "expected int literal")),
            }
        } else {
            // Could also silently ignore the unexpected attribute by returning `Ok(None)`
            return Err(syn::Error::new_spanned(
                &name_value.path,
                "unsupported tag attribute, expected `offset` or `size`",
            ));
        }
    }

    if offset.is_none() {
        return Err(syn::Error::new_spanned(
            &meta_list.path,
            "tag attribute `offset` is required",
        ));
    }

    Ok(TagInfo {
        offset: offset.unwrap(),
        size,
    })
}

pub struct TagInfo {
    pub offset: u32,
    pub size: Option<u32>,
}
