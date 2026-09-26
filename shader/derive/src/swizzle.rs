use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

/// Generate all the WGSL vector component selections and swizzles, for both the xyzw and rgba
/// component names (they can not be mixed in WGSL).
pub fn impl_shader_swizzles_impl() -> TokenStream {
  let impls = [2_u8, 3, 4].map(|size| {
    let vec = format_ident!("Vec{}", size);
    let methods = ["xyzw", "rgba"].into_iter().flat_map(|names| {
      let names = &names.as_bytes()[..size as usize];
      (1..=4).flat_map(move |len| {
        let count = (size as usize).pow(len);
        (0..count).map(move |index| {
          let mut index = index;
          let components: Vec<_> = (0..len)
            .map(|_| {
              let component = (index % size as usize) as u8;
              index /= size as usize;
              component
            })
            .rev()
            .collect();
          swizzle_method(names, &components)
        })
      })
    });

    quote! {
      impl<T: ShaderScalarType> Node<#vec<T>> {
        #(#methods)*
      }
    }
  });

  quote! { #(#impls)* }
}

fn swizzle_method(names: &[u8], components: &[u8]) -> TokenStream {
  let name: String = components
    .iter()
    .map(|c| names[*c as usize] as char)
    .collect();
  let name = format_ident!("{}", name);
  let components = components.iter().map(|c| Literal::u8_unsuffixed(*c));

  if components.len() == 1 {
    quote! {
      #[inline(always)]
      pub fn #name(&self) -> Node<T> {
        self.component::<#(#components)*>()
      }
    }
  } else {
    let len = components.len();
    let output = format_ident!("Vec{}", len);
    let swizzle = format_ident!("swizzle{}", len);
    quote! {
      #[inline(always)]
      pub fn #name(&self) -> Node<#output<T>> {
        self.#swizzle::<#(#components),*>()
      }
    }
  }
}

#[test]
fn all_swizzles_are_generated() {
  let code = impl_shader_swizzles_impl().to_string();
  let count = code.matches("pub fn").count();
  // sum of size^len for len in 1..=4, for both the xyzw and rgba names
  let expect: usize = [2_usize, 3, 4]
    .iter()
    .map(|size| (1..=4).map(|len| size.pow(len)).sum::<usize>())
    .sum();
  assert_eq!(count, expect * 2);
  assert!(code.contains("pub fn wzyx"));
  assert!(code.contains("pub fn abgr"));
  assert!(code.contains("swizzle4 :: < 3 , 2 , 1 , 0 >"));
}
