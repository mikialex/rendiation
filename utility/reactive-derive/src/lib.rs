use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Type;

// note, this should split to another crate
#[proc_macro_attribute]
pub fn global_registered_collection(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input: syn::ItemFn = syn::parse2(input.into()).unwrap();

  let mut original_fn = input;
  let name = original_fn.sig.ident;
  let new_name = format_ident!("{name}_inner");
  let rt = &original_fn.sig.output;

  original_fn.sig.ident = new_name.clone();

  quote! {
    pub fn #name() #rt {
      reactive::global_collection_registry().fork_or_insert_with(#new_name)
    }

    #original_fn
  }
  .into()
}

fn get_ty_name_pair(rt: &syn::ReturnType) -> (syn::Type, syn::Type) {
  let trait_token = match rt {
    syn::ReturnType::Type(_, ty) => match ty.as_ref() {
      Type::ImplTrait(im) => {
        let ty = &im.bounds[0];
        match ty {
          syn::TypeParamBound::Trait(t) => t.path.clone(),
          _ => unreachable!(),
        }
      }
      _ => unreachable!(),
    },
    _ => unreachable!(),
  };
  let trait_token = &trait_token.segments[0];
  let p = match &trait_token.arguments {
    syn::PathArguments::AngleBracketed(p) => p,
    _ => unreachable!(),
  };
  let args_k = match &p.args[0] {
    syn::GenericArgument::Type(ty) => ty.clone(),
    _ => unreachable!(),
  };
  let args_v = match &p.args[1] {
    syn::GenericArgument::Type(ty) => ty.clone(),
    _ => unreachable!(),
  };

  (args_k, args_v)
}

#[proc_macro_attribute]
pub fn global_registered_collection_and_many_one_idx_relation(
  _args: TokenStream,
  input: TokenStream,
) -> TokenStream {
  let input: syn::ItemFn = syn::parse2(input.into()).unwrap();

  let mut original_fn = input;
  let name = original_fn.sig.ident;
  let new_name = format_ident!("{name}_inner");
  let rt = &original_fn.sig.output;
  let (args_k, args_v) = get_ty_name_pair(rt);

  original_fn.sig.ident = new_name.clone();
  original_fn.vis = syn::Visibility::Inherited;

  let relation_fn_name = format_ident!("{name}_many_one_relation");

  quote! {
    pub fn #name() #rt + Clone {
      reactive::global_collection_registry().fork_or_insert_with(#new_name)
    }

    pub fn #relation_fn_name() -> impl ReactiveOneToManyRelation<#args_v, #args_k> + Clone {
      reactive::global_collection_registry().get_or_create_relation_by_idx(#new_name)
    }

    #original_fn
  }
  .into()
}

#[proc_macro_attribute]
pub fn global_registered_collection_and_many_one_hash_relation(
  _args: TokenStream,
  input: TokenStream,
) -> TokenStream {
  let input: syn::ItemFn = syn::parse2(input.into()).unwrap();

  let mut original_fn = input;
  let name = original_fn.sig.ident;
  let new_name = format_ident!("{name}_inner");
  let rt = &original_fn.sig.output;
  let (args_k, args_v) = get_ty_name_pair(rt);

  original_fn.sig.ident = new_name.clone();
  original_fn.vis = syn::Visibility::Inherited;

  let relation_fn_name = format_ident!("{name}_many_one_relation");

  quote! {
    pub fn #name() #rt + Clone {
      reactive::global_collection_registry().fork_or_insert_with(#new_name)
    }

    pub fn #relation_fn_name() -> impl ReactiveOneToManyRelation<#args_v, #args_k> + Clone {
      reactive::global_collection_registry().get_or_create_relation_by_hash(#new_name)
    }

    #original_fn
  }
  .into()
}
