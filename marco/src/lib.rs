extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(BindGroup)]
pub fn derive_lens(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_bindgroup_impl(input)
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

use quote::quote;
use syn::{spanned::Spanned, Data};

pub(crate) fn derive_bindgroup_impl(
  input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
  match &input.data {
    Data::Struct(_) => derive_struct(&input),
    Data::Enum(e) => Err(syn::Error::new(
      e.enum_token.span(),
      "Bindgroup implementations cannot be derived from enums",
    )),
    Data::Union(u) => Err(syn::Error::new(
      u.union_token.span(),
      "Bindgroup implementations cannot be derived from unions",
    )),
  }
}

fn derive_struct(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
  // todo!()
  let struct_name = &input.ident;
  let struct_generic = &input.generics;
  let static_name_string = struct_name.to_string() + "_bindgroup_layout";
  let static_bindgroup_name = proc_macro2::Ident::new(&static_name_string, proc_macro2::Span::call_site());
  
  let struct_bindgroup_static = {
    quote! {
        static mut #static_bindgroup_name : Option<rendiation::BindGroupLayout> = None;
    }
  };

  let result = quote! {
    #struct_bindgroup_static

    impl #struct_generic rendiation::BindGroupProvider for #struct_name #struct_generic {
        fn provide_layout(renderer: &rendiation::WGPURenderer) -> &'static rendiation::BindGroupLayout {
          unsafe {
            if let Some(layout) = &#static_bindgroup_name {
              &layout
            } else {
              let builder = rendiation::BindGroupLayoutBuilder::new()
                .bind_texture2d(rendiation::ShaderType::Fragment)
                .bind_sampler(rendiation::ShaderType::Fragment);
              let layout = renderer
                .device
                .create_bind_group_layout(&rendiation::BindGroupLayoutDescriptor {
                  bindings: &builder.bindings,
                });
                #static_bindgroup_name = Some(layout);
                #static_bindgroup_name.as_ref().unwrap()
            }
          }
        }
      
        fn create_bindgroup(&self, renderer: &rendiation::WGPURenderer) -> rendiation::WGPUBindGroup {
          rendiation::BindGroupBuilder::new()
            .texture(self.texture)
            .sampler(self.sampler)
            .build(&renderer.device, CopyParam::provide_layout(renderer))
        }
      }
      
  };

  Ok(result)
  // todo!()

  // quote! {
  //     impl druid::Lens<#ty, #field_ty> for #twizzled_name::#field_name {
  //         fn with<V, F: FnOnce(&#field_ty) -> V>(&self, data: &#ty, f: F) -> V {
  //             f(&data.#field_name)
  //         }

  //         fn with_mut<V, F: FnOnce(&mut #field_ty) -> V>(&self, data: &mut #ty, f: F) -> V {
  //             f(&mut data.#field_name)
  //         }
  //     }
  // }
}
