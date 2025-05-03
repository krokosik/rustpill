extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Block, ImplItem, Item, ReturnType, Signature, Stmt, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn blocking_async(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_item = parse_macro_input!(item as Item);

    match parsed_item {
        Item::Fn(mut func) => {
            if func.sig.asyncness.is_none() {
                let error_message =
                    "blocking_async attribute on a function requires the function to be async";
                return syn::Error::new_spanned(&func.sig, error_message)
                    .to_compile_error()
                    .into();
            }
            transform_async_fn(&mut func.sig, &mut func.block);
            quote! { #func }.into()
        }
        Item::Impl(mut imp) => {
            for item in &mut imp.items {
                if let ImplItem::Fn(method) = item {
                    if method.sig.asyncness.is_some() {
                        transform_async_fn(&mut method.sig, &mut method.block);
                    }
                }
            }
            // Return the potentially modified impl block
            quote! { #imp }.into()
        }
        _ => TokenStream::from(quote! { #parsed_item }),
    }
}

// Modify function signature to accept Signature and Block directly
fn transform_async_fn(sig: &mut Signature, block: &mut Block) {
    // Remove the async keyword from the signature
    sig.asyncness = None;

    let stmts = &block.stmts;

    // Check return type from the signature
    let is_result = match &sig.output {
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map_or(false, |seg| seg.ident == "Result")
            } else {
                false
            }
        }
        ReturnType::Default => false,
    };

    if is_result {
        if let Some((last_stmt, other_stmts)) = stmts.split_last() {
            if let Stmt::Expr(expr, None) = last_stmt {
                // Modify the block directly
                *block = syn::parse_quote! {
                    {
                        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
                            #(#other_stmts)*
                            let res = #expr;
                            Ok(res)
                        })
                    }
                };
            } else {
                *block = syn::parse_quote! {
                    {
                        pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
                            #(#stmts)*
                        })
                    }
                };
            }
        } else {
            *block = syn::parse_quote! {
                {
                    pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
                        Ok(())
                    })
                }
            };
        }
    } else {
        *block = syn::parse_quote! {
            {
                pyo3_async_runtimes::tokio::get_runtime().block_on(async move {
                    #(#stmts)*
                })
            }
        };
    }
}
