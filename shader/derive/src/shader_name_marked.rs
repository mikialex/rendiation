use proc_macro::TokenStream;
use quote::ToTokens;
use syn::fold::Fold;

struct ShaderNameMarkFolder;

fn binding_ident_of(pat: &syn::Pat) -> Option<&syn::Ident> {
  match pat {
    syn::Pat::Ident(pat_ident) if pat_ident.subpat.is_none() && pat_ident.by_ref.is_none() => {
      Some(&pat_ident.ident)
    }
    syn::Pat::Type(pat_ty) => binding_ident_of(&pat_ty.pat),
    _ => None,
  }
}

impl Fold for ShaderNameMarkFolder {
  fn fold_item_fn(&mut self, item: syn::ItemFn) -> syn::ItemFn {
    item
  }

  fn fold_block(&mut self, mut block: syn::Block) -> syn::Block {
    let mut stmts = Vec::with_capacity(block.stmts.len() * 2);

    for stmt in block.stmts {
      let stmt = self.fold_stmt(stmt);

      let marked = if let syn::Stmt::Local(local) = &stmt
        && local.init.is_some()
        && let Some(ident) = binding_ident_of(&local.pat)
      {
        let label = syn::LitStr::new(&ident.to_string(), ident.span());
        Some(syn::parse_quote!(
          rendiation_shader_api::DebugLabelMarkExt::mark_label(&#ident, #label);
        ))
      } else {
        None
      };

      stmts.push(stmt);
      if let Some(marked) = marked {
        stmts.push(marked);
      }
    }

    block.stmts = stmts;
    block
  }
}

pub(crate) fn mark_all_lets_in_block(block: syn::Block) -> syn::Block {
  ShaderNameMarkFolder.fold_block(block)
}

pub fn shader_name_marked_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
  let mut item = syn::parse_macro_input!(input as syn::ItemFn);
  *item.block = mark_all_lets_in_block(*item.block);
  item.into_token_stream().into()
}
