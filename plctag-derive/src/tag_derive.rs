use proc_macro2::{Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use proc_quote::quote;
use syn::{Attribute, Data, DataStruct, DeriveInput, Fields, Ident, Index, Lit, Meta, NestedMeta};

pub fn expand_tag_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let plctag = match crate_name("plctag").or_else(|_| crate_name("plctag_core")) {
        Ok(found) => match found {
            FoundCrate::Itself => Ident::new("crate", Span::call_site()),
            FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        },
        Err(_) => Ident::new("crate", Span::call_site()),
    };
    let fields = match input.data {
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
            let field_name = f.ident;
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

    let gets = items
        .iter()
        .map(|(field_name, i)| {
            let index = Index::from(*i as usize);
            Ok(quote! {
                #plctag::GetValue(&mut  self.#field_name, tag, offset + #index)?;
            })
        })
        .collect::<syn::Result<TokenStream>>()?;

    let sets = items
        .iter()
        .map(|(field_name, i)| {
            let index = Index::from(*i as usize);
            Ok(quote! {
                #plctag::SetValue(&mut  self.#field_name, tag, offset + #index)?;
            })
        })
        .collect::<syn::Result<TokenStream>>()?;

    let st_name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #st_name #ty_generics #where_clause {}

        impl  #impl_generics #plctag::GetValue for $st_name #ty_generics #where_clause{
            fn get_value(&mut self, tag: &#plctag::RawTag, offset: u32) -> #plctag::Result<()>{
                #gets
                Ok(())
            }
        }

        impl  #impl_generics #plctag::SetValue for $st_name #ty_generics #where_clause{
            fn get_value(&mut self, tag: &#plctag::RawTag, offset: u32) -> #plctag::Result<()>{
                #sets
                Ok(())
            }
        }
    })
}

fn get_offset_attr(attr: &Attribute) -> syn::Result<u32> {
    let meta = attr.parse_meta()?;
    //offset()
    let meta_list = match meta {
        Meta::List(list) => list,
        _ => {
            return Err(syn::Error::new_spanned(
                meta,
                "expected a list-style attribute",
            ))
        }
    };

    //offset(i), here i
    let nested = match meta_list.nested.len() {
        1 => &meta_list.nested[0],
        _ => {
            return Err(syn::Error::new_spanned(
                meta_list.nested,
                "currently only a single tag attribute is supported",
            ));
        }
    };

    let offset_value = match nested {
        NestedMeta::Lit(offset_value) => offset_value,
        _ => {
            return Err(syn::Error::new_spanned(
                meta_list.nested,
                "currently only a single tag attribute is supported",
            ));
        }
    };

    match &offset_value {
        Lit::Int(s) => {
            // Parse string contents to `Ident`, reporting an error on the string
            // literal's span if parsing fails
            s.base10_parse()
        }
        lit => Err(syn::Error::new_spanned(lit, "expected int literal")),
    }
}
