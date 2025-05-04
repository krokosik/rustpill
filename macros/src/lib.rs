extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ImplItem, Item, parse_macro_input};

#[proc_macro_attribute]
pub fn blocking_async(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_item = parse_macro_input!(item as Item);

    match parsed_item {
        Item::Impl(mut imp) => {
            for item in &mut imp.items {
                if let ImplItem::Fn(method) = item {
                    if method.sig.asyncness.is_some() {
                        method.sig.asyncness = None;
                        let stmts = &method.block.stmts;
                        method.block = syn::parse_quote! {
                            {
                                pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
                                    #(#stmts)*
                                })
                            }
                        };
                    }
                }
            }
            // Return the potentially modified impl block
            quote! { #imp }.into()
        }
        _ => TokenStream::from(quote! { #parsed_item }),
    }
}
