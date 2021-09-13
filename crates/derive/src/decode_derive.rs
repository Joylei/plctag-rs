// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

use crate::shared::{get_crate, get_fields};
use proc_macro2::TokenStream;
use proc_quote::quote;
use syn::{DeriveInput, Index};

pub fn expand_tag_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let plctag = get_crate()?;
    let items = get_fields(input.data)?;

    let gets = items
        .iter()
        .map(|(field_name, ty, i)| {
            let index = Index::from(i.offset as usize);
            Ok(quote! {
                res.#field_name = #ty::decode(tag, offset + #index)?;
            })
        })
        .collect::<syn::Result<TokenStream>>()?;

    let st_name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl  #impl_generics #plctag::Decode for #st_name #ty_generics #where_clause
         {
            fn decode(tag: &#plctag::RawTag, offset: u32) -> #plctag::Result<Self>{
                use #plctag::Decode;

                let mut res = Self::default();
                #gets
                Ok(res)
            }
        }
    })
}
