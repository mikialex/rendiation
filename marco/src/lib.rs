extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(BindGroup, attributes(bind_type))]
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

enum BindGroupType {
  Texture2d,
  Sampler,
}

enum ShaderType {
  Fragment,
  Vertex,
}

fn derive_struct(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
  let struct_name = &input.ident;
  let struct_generic = &input.generics;
  let static_name_string = struct_name.to_string() + "_bindgroup_layout";
  let static_bindgroup_name =
    proc_macro2::Ident::new(&static_name_string, proc_macro2::Span::call_site());

  let struct_bindgroup_static = {
    quote! {
        static mut #static_bindgroup_name : Option<rendiation::BindGroupLayout> = None;
    }
  };

  let fields = if let syn::Data::Struct(syn::DataStruct {
    fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
    ..
  }) = input.data
  {
    named
  } else {
    return Err(syn::Error::new(
      input.span(),
      "BindGroup implementations can only be derived from structs with named fields",
    ));
  };

  let defs = fields.iter().filter_map(|f| {
    let field_name = &f.ident;

    let attr = f.attrs.iter().find(|a| a.path.is_ident("bind_type"))?;

    let parse = match attr.parse_meta() {
      Ok(syn::Meta::NameValue(nv)) => Some(nv),
      Ok(_) => None,
      Err(_) => None,
    };

    let tag = match parse?.lit {
      syn::Lit::Str(s) => Some(s.value()),
      _ => None,
    }?;

    let tags: Vec<&str> = tag.split(':').collect();
    if tags.len() != 2 {
      return None
    }
    let shader_type = match tags[1] {
      "fragment" => quote! {rendiation::ShaderType::Fragment},
      "vertex" => quote! {rendiation::ShaderType::Vertex},
      _ => {return None}
    };

    let re = match tags[0] {
      "uniform-buffer" =>  Some((
        quote! {.bind_uniform_buffer(#shader_type)},
        quote! {.buffer(self.#field_name)},
      )),
      "texture2d" => Some((
        quote! {.bind_texture2d(#shader_type)},
        quote! {.texture(self.#field_name)},
      )),
      "sampler" =>  Some((
        quote! {.bind_sampler(#shader_type)},
        quote! {.sampler(self.#field_name)},
      )),
      _ =>  None
    };
    re
  });

  let mut layout_build = Vec::new();
  let mut bg_build = Vec::new();
  for v in defs{
    layout_build.push(v.0);
    bg_build.push(v.1);
  }

  let result = quote! {
    #struct_bindgroup_static

    impl #struct_generic rendiation::BindGroupProvider for #struct_name #struct_generic {
        fn provide_layout(renderer: &rendiation::WGPURenderer) -> &'static rendiation::BindGroupLayout {
          unsafe {
            if let Some(layout) = &#static_bindgroup_name {
              &layout
            } else {
              let builder = rendiation::BindGroupLayoutBuilder::new()
                #(#layout_build)*;
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
            #(#bg_build)*
            .build(&renderer.device,  #struct_name::provide_layout(renderer))
        }
      }

  };

  Ok(result)
}
