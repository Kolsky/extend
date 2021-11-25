use proc_macro_error::*;
use proc_macro::TokenStream;
use quote::*;
use syn::FnArg;
use syn::Pat;
use syn::spanned::Spanned;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn ext(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn: syn::ItemFn = syn::parse_macro_input!(item);
    let (self_ty, self_mut) = self_arg_ty_mut(&item_fn);
    let item_fn_in_mod = make_mod_fn(item_fn.clone());
    let vis = &item_fn.vis;
    let fn_name = &item_fn.sig.ident;
    let inputs = item_fn.sig.inputs.iter().skip(1);
    let input_pats = inputs.clone().map(untype_input);
    let input_tys = inputs.clone().filter_map(|arg| match arg { FnArg::Typed(t) => Some(&t.ty), | _ => None });
    let impl_pats = inputs.map(unident_unused_input);
    let impl_tys = input_tys.clone();
    let output = &item_fn.sig.output;
    let fn_block = &item_fn.block;
    let (ig, tg, wc) = &item_fn.sig.generics.split_for_impl();
    let mut wc = wc.cloned();
    if let Some(wc) = &mut wc {
        wc.predicates.push(syn::parse_quote!(Self: Sized));
    }
    let output = quote! {
        #item_fn_in_mod
        #[allow(non_camel_case_types)]
        #vis trait #fn_name #ig #wc {
            fn #fn_name(self, #(#input_pats: #input_tys),*) #output;
        }
        impl #ig #fn_name #tg for #self_ty #wc {
            fn #fn_name(#self_mut self, #(#impl_pats: #impl_tys),*) #output #fn_block
        }
    };
    output.into()
}

fn make_mod_fn(mut item_fn: syn::ItemFn) -> syn::ItemFn {
    for this in item_fn.sig.inputs.first_mut() {
        if let syn::FnArg::Typed(this) = this {
            if let syn::Pat::Ident(this) = &mut *this.pat {
                this.ident = syn::parse_quote!(this);
            }
        }
    }
    let fn_name = &item_fn.sig.ident;
    for input in item_fn.sig.inputs.iter_mut().skip(1) {
        let pat = untype_input(input);
        if let FnArg::Typed(t) = input {
            t.pat = pat;
        }
    }
    let args = item_fn.sig.inputs.iter().skip(1).map(untype_input);
    
    item_fn.block = syn::parse_quote!({
        this.#fn_name(#(#args),*)
    });
    item_fn
}

fn self_arg_ty_mut(item_fn: &syn::ItemFn) -> (&syn::Type, Option<syn::token::Mut>) {
    let first_arg = item_fn.sig.receiver()
        .unwrap_or_else(|| abort_call_site!("First argument must be self"));
    let first_arg_typed = match first_arg {
        FnArg::Receiver(r) => abort!(r.span(), "Indicate type of self"),
        FnArg::Typed(t) => t,
    };
    let mutable = matches!(&*first_arg_typed.pat, Pat::Ident(i) if i.mutability.is_some());
    let mutability = mutable.then(|| syn::Token![mut](first_arg_typed.span()));
    (&first_arg_typed.ty, mutability)
}

fn extract_typed_pat(input: &FnArg) -> Box<Pat> {
    match input.clone() {
        FnArg::Receiver(r) => abort!(r.span(), "Missing type ascription for argument"),
        FnArg::Typed(t) => t.pat,
    }
}

fn untype_input(input: &syn::FnArg) -> Box<Pat> {
    let mut pat = extract_typed_pat(input);
    loop { break match *pat {
        Pat::Ident(ref mut i) => {
            i.attrs.clear();
            i.by_ref.take();
            i.mutability.take();
            i.subpat.take();
            pat
        }
        Pat::Reference(r) => {
            pat = r.pat;
            continue;
        }
        _ => abort!(pat.span(), "Unnamed arguments are not allowed, consider using @-bindings"),
    }}
}

fn unident_unused_input(input: &syn::FnArg) -> Box<Pat> {
    let mut pat = extract_typed_pat(input);
    let mut pat_mut = &mut *pat;
    loop { break match pat_mut {
        Pat::Ident(i) => {
            if i.subpat.is_some() && i.ident.to_string().starts_with('_') {
                i.subpat.take().map(|(_, pat)| *pat_mut = *pat);
            }
        }
        Pat::Reference(r) => {
            pat_mut = &mut r.pat;
            continue;
        }
        _ => (),
    }}
    pat
}