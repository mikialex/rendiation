use crate::*;

pub fn shadergraph_fn_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
  let func = parse_macro_input!(input as syn::ItemFn);

  let sig = &func.sig;
  let vis = &func.vis;
  let params = &sig.inputs;
  let rt = &sig.output;
  let origin_fn = &sig.ident;
  let fn_ident = quote::format_ident!("{}_fn", origin_fn);
  let block = &func.block;

  let input_nodes: Vec<_> = sig
    .inputs
    .iter()
    .filter_map(|param| match param {
      syn::FnArg::Typed(param) => {
        let ident = match param.pat.as_ref() {
            syn::Pat::Ident(ident) => ident,
            _ => return None
        };
        Some((&ident.ident, &param.ty))
      },
      _ => None,
    })
    .map(|(name, ty)| {
      let name = quote::format_ident!("n_{}", name);
      quote::quote! {let #name = push_fn_parameter::<<#ty as shadergraph::ProcMacroNodeHelper>::NodeType>(); }
    })
    .collect();

  let names: Vec<_> = sig
    .inputs
    .iter()
    .filter_map(|param| match param {
      syn::FnArg::Typed(param) => match param.pat.as_ref() {
        syn::Pat::Ident(ident) => Some(&ident.ident),
        _ => None,
      },
      _ => None,
    })
    .map(|name| {
      let name = quote::format_ident!("n_{}", name);
      quote::quote! { #name, }
    })
    .collect();

  let rt_define = match rt {
    syn::ReturnType::Default => quote::quote! {},
    syn::ReturnType::Type(_, ty) => {
      quote::quote! { end_fn_define(<<#ty as shadergraph::ProcMacroNodeHelper>::NodeType>::TYPE.into()) }
    }
  };

  let real_input_call: Vec<_> = sig
    .inputs
    .iter()
    .filter_map(|param| match param {
      syn::FnArg::Typed(param) => match param.pat.as_ref() {
        syn::Pat::Ident(ident) => Some(&ident.ident),
        _ => None,
      },
      _ => None,
    })
    .map(|name| {
      quote::quote! { #name.handle(), }
    })
    .collect();

  quote::quote! {
    #vis fn #fn_ident(#params) #rt {
      let f_meta = begin_define_fn(std::any::type_name_of_val(&#origin_fn).to_string()).unwrap_or_else(|| {
        #(#input_nodes)*
        #origin_fn(#(#names)*);
        #rt_define
      });

      unsafe { shader_fn_call(f_meta.clone(), vec![#(#real_input_call)*]).into_node() }
    }
    #sig #block
  }
  .into()
}
