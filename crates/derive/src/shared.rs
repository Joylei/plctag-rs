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
                .filter(|attr| attr.path.is_ident("tag"))
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

    let nested = &meta_list.nested;
    let mut offset = None;
    let mut size = None;
    let mut encode_fn = None;
    let mut decode_fn = None;
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
        } else if name_value.path.is_ident("encode_fn") {
            match &name_value.lit {
                Lit::Str(s) => {
                    if encode_fn.is_some() {
                        return Err(syn::Error::new_spanned(
                            s,
                            "redundant definition for encode_fn",
                        ));
                    }
                    let expr: syn::ExprPath = s.parse()?;
                    encode_fn = Some(expr);
                }
                lit => return Err(syn::Error::new_spanned(lit, "expected Str literal")),
            }
        } else if name_value.path.is_ident("decode_fn") {
            match &name_value.lit {
                Lit::Str(s) => {
                    if decode_fn.is_some() {
                        return Err(syn::Error::new_spanned(
                            s,
                            "redundant definition for decode_fn",
                        ));
                    }
                    let expr: syn::ExprPath = s.parse()?;
                    decode_fn = Some(expr);
                }
                lit => return Err(syn::Error::new_spanned(lit, "expected Str literal")),
            }
        } else {
            // Could also silently ignore the unexpected attribute by returning `Ok(None)`
            return Err(syn::Error::new_spanned(
                &name_value.path,
                "unknown tag attribute",
            ));
        }
    }

    if ctx.is_encode && encode_fn.is_none() && offset.is_none() {
        return Err(syn::Error::new_spanned(
            &meta_list.path,
            "at least one of tag attribute `offset`, `encode_fn` is required",
        ));
    } else if !ctx.is_encode && decode_fn.is_none() && offset.is_none() {
        return Err(syn::Error::new_spanned(
            &meta_list.path,
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
