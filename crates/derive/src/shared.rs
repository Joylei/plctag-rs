// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use syn::{Attribute, Data, DataStruct, Fields, Ident, LitInt, LitStr, Type};

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

pub fn get_fields(data: Data, ctx: &Context) -> syn::Result<Vec<(Ident, Type, TagAttr)>> {
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
                .filter(|attr| attr.path().is_ident("tag"))
                .collect();
            assert!(!attrs.is_empty());
            let offset = match attrs.len() {
                0 => return Ok(None),
                1 => get_tag_attr(attrs[0], ctx)?,
                _ => {
                    let mut error =
                        syn::Error::new_spanned(attrs[1], "redundant `tag()` attribute");
                    error.combine(syn::Error::new_spanned(attrs[0], "note: first one here"));
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

fn get_tag_attr(attr: &Attribute, ctx: &Context) -> syn::Result<TagAttr> {
    let mut offset = None;
    let mut size = None;
    let mut encode_fn = None;
    let mut decode_fn = None;

    attr.parse_nested_meta(|item| {
        if item.path.is_ident("offset") {
            if offset.is_some() {
                return Err(item.error("redundant definition for offset"));
            }
            let lit: LitInt = item.value()?.parse()?;
            offset = Some(lit.base10_parse()?);
        } else if item.path.is_ident("size") {
            if size.is_some() {
                return Err(item.error("redundant definition for size"));
            }
            let lit: LitInt = item.value()?.parse()?;
            size = Some(lit.base10_parse()?);
        } else if item.path.is_ident("encode_fn") {
            if encode_fn.is_some() {
                return Err(item.error("redundant definition for encode_fn"));
            }
            let lit: LitStr = item.value()?.parse()?;
            let expr: syn::ExprPath = lit.parse()?;
            encode_fn = Some(expr);
        } else if item.path.is_ident("decode_fn") {
            if decode_fn.is_some() {
                return Err(item.error("redundant definition for decode_fn"));
            }
            let lit: LitStr = item.value()?.parse()?;
            let expr: syn::ExprPath = lit.parse()?;
            decode_fn = Some(expr);
        } else {
            return Err(item.error("unknown tag attr"));
        }
        Ok(())
    })?;

    if ctx.is_encode && encode_fn.is_none() && offset.is_none() {
        return Err(syn::Error::new_spanned(
            attr.path(),
            "at least one of tag attribute `offset`, `encode_fn` is required",
        ));
    } else if !ctx.is_encode && decode_fn.is_none() && offset.is_none() {
        return Err(syn::Error::new_spanned(
            attr.path(),
            "at least one of tag attribute `offset`, `decode_fn` is required",
        ));
    }

    Ok(TagAttr {
        offset,
        size,
        encode_fn,
        decode_fn,
    })
}

pub struct TagAttr {
    pub offset: Option<u32>,
    pub size: Option<u32>,
    pub encode_fn: Option<syn::ExprPath>,
    pub decode_fn: Option<syn::ExprPath>,
}

pub struct Context {
    pub is_encode: bool,
}
