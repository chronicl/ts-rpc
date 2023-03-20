use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

#[proc_macro_attribute]
pub fn ts_export(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match ts_export_inner(TokenStream::from(attr), TokenStream::from(input)) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn ts_export_inner(_: TokenStream, input: TokenStream) -> Result<TokenStream, syn::Error> {
    let f = syn::parse2::<syn::ItemFn>(input)?;

    // Check if the function is async
    if f.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(&f.sig.ident, "Must be async"));
    }
    if !f.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &f.sig.ident,
            "Generics not supported",
        ));
    }

    let signature = &f.sig;
    let name = &signature.ident;
    let static_name = Ident::new(&format!("__{}", name), Span::call_site());

    let mut input_types = Vec::new();
    let mut input_type_names = Vec::new();
    for i in &signature.inputs {
        match i {
            syn::FnArg::Receiver(_) => {
                return Err(syn::Error::new_spanned(
                    &f.sig.ident,
                    "self argument not supported",
                ));
            }
            syn::FnArg::Typed(typed) => {
                input_types.push((*(typed.ty)).clone());
                input_type_names.push((*(typed.pat)).clone());
            }
        }
    }

    // removing axum parameter
    let is_axum = input_types
        .last()
        .map(|ty| {
            if let syn::Type::Path(path) = ty {
                for segment in path.path.segments.iter() {
                    if segment.ident == "Axum" {
                        return true;
                    }
                }
            }
            false
        })
        .unwrap_or(false);
    if is_axum {
        input_types.pop();
        input_type_names.pop();
    }

    let output = match &signature.output {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, ty) => quote!(#ty),
    };

    let this_crate = get_crate_name("ts_rpc", false);

    Ok(quote!(
        #f

        static #static_name: #this_crate::once_cell::sync::Lazy<#this_crate::TsFn> = #this_crate::once_cell::sync::Lazy::new(|| {
            let mut ts = #this_crate::TsFn::new(stringify!(#name));
            #(
                ts.add_request_type::<#input_types>(stringify!(#input_type_names));
            )*
            ts.set_response_type::<#output>();
            ts
        });
        #this_crate::inventory::submit! {
            #this_crate::LazyTsFn(&#static_name)
        }
    ))
}

#[test]
fn test() {
    let input = quote! {
        async fn login(email: String, password: String, axum: ts_rc::Axum<String>) -> String {}
    };
    let output = ts_export_inner(TokenStream::new(), input).unwrap();
    println!("{}", rustfmt(output.to_string()));
}

#[allow(dead_code)]
fn rustfmt(s: impl AsRef<[u8]>) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(s.as_ref()).unwrap();
    }

    String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
}

pub(crate) fn get_crate_name(name: &str, internal: bool) -> TokenStream {
    use proc_macro_crate::{crate_name, FoundCrate};
    if internal {
        quote! { crate }
    } else {
        let name = match crate_name(name) {
            Ok(FoundCrate::Name(name)) => name,
            Ok(FoundCrate::Itself) | Err(_) => name.to_string(),
        };
        let name = Ident::new(&name, Span::call_site());
        quote!(#name)
    }
}

mod derive;

// #[proc_macro_derive(TS2, attributes(ts))]
// pub fn typescript(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     match derive::typescript(TokenStream::from(input)) {
//         Err(err) => err.to_compile_error(),
//         Ok(result) => result,
//     }
//     .into()
// }
