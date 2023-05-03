// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

use crate::shared::{get_crate, get_fields, Context};
use proc_macro2::TokenStream;
use proc_quote::quote;
use syn::{DeriveInput, Index};

pub fn expand_tag_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let ctx = Context { is_encode: true };
    let plctag = get_crate()?;
    let items = get_fields(input.data, &ctx)?;

    let sets = items
        .iter()
        .map(|(field_name, _ty, attr)| {
            let ts = match attr.encode_fn {
                Some(ref f) => quote! {
                    #f(&self.#field_name, tag, offset)?;
                },
                None => {
                    let index = Index::from(attr.offset.unwrap() as usize);
                    quote! {
                        #plctag::Encode::encode(&self.#field_name, tag, offset + #index)?;
                    }
                }
            };
            Ok(ts)
        })
        .collect::<syn::Result<TokenStream>>()?;

    let st_name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl  #impl_generics #plctag::Encode for #st_name #ty_generics #where_clause{
            fn encode(&self, tag: &#plctag::RawTag, offset: u32) -> #plctag::Result<()>{
                #sets
                Ok(())
            }
        }
    })
}
