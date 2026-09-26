use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

/// Generate all the WGSL vector component selections and swizzles, for both the xyzw and rgba
/// component names (they can not be mixed in WGSL):
/// - the value side `Node<VecN<T>>`, all the swizzles of 1 to 4 components
/// - the writable pointer `DirectPrimitivePtrView<VecN<T>>`, the component references and the
///   writable swizzle views, whose components must be distinct
/// - the readonly pointer `ReadonlyDirectPrimitivePtrView<VecN<T>>`, the component references
pub fn impl_shader_swizzles_impl() -> TokenStream {
  let impls = [2_u8, 3, 4].map(|size| {
    let vec = format_ident!("Vec{}", size);
    let swizzles: Vec<_> = ["xyzw", "rgba"]
      .into_iter()
      .flat_map(|names| {
        let names = &names.as_bytes()[..size as usize];
        (1..=4).flat_map(move |len| all_components(size, len).map(move |c| (names, c)))
      })
      .collect();

    let value_methods = swizzles
      .iter()
      .map(|(names, components)| value_method(names, components));

    let ptr_methods = swizzles
      .iter()
      .filter(|(_, components)| is_distinct(components))
      .map(|(names, components)| ptr_method(names, components, &vec));

    let readonly_ptr_methods = swizzles
      .iter()
      .filter(|(_, components)| components.len() == 1)
      .map(|(names, components)| {
        let name = method_name(names, components);
        let component = Literal::u8_unsuffixed(components[0]);
        quote! {
          #[inline(always)]
          pub fn #name(&self) -> ShaderReadonlyPtrOf<T> {
            self.component::<#component>()
          }
        }
      });

    quote! {
      impl<T: ShaderScalarType> Node<#vec<T>> {
        #(#value_methods)*
      }
      impl<T: ShaderScalarType> DirectPrimitivePtrView<#vec<T>> {
        #(#ptr_methods)*
      }
      impl<T: ShaderScalarType> ReadonlyDirectPrimitivePtrView<#vec<T>> {
        #(#readonly_ptr_methods)*
      }
    }
  });

  quote! { #(#impls)* }
}

/// all the component index lists of the given length, each index is less than the size
fn all_components(size: u8, len: u32) -> impl Iterator<Item = Vec<u8>> {
  let count = (size as usize).pow(len);
  (0..count).map(move |mut index| {
    let mut components: Vec<_> = (0..len)
      .map(|_| {
        let component = (index % size as usize) as u8;
        index /= size as usize;
        component
      })
      .collect();
    components.reverse();
    components
  })
}

fn is_distinct(components: &[u8]) -> bool {
  components
    .iter()
    .enumerate()
    .all(|(i, c)| !components[i + 1..].contains(c))
}

fn method_name(names: &[u8], components: &[u8]) -> proc_macro2::Ident {
  let name: String = components
    .iter()
    .map(|c| names[*c as usize] as char)
    .collect();
  format_ident!("{}", name)
}

fn value_method(names: &[u8], components: &[u8]) -> TokenStream {
  let name = method_name(names, components);
  let len = components.len();
  let components = components.iter().map(|c| Literal::u8_unsuffixed(*c));

  if len == 1 {
    quote! {
      #[inline(always)]
      pub fn #name(&self) -> Node<T> {
        self.component::<#(#components)*>()
      }
    }
  } else {
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

fn ptr_method(names: &[u8], components: &[u8], vec: &proc_macro2::Ident) -> TokenStream {
  let name = method_name(names, components);
  let len = components.len();
  let components = components.iter().map(|c| Literal::u8_unsuffixed(*c));

  if len == 1 {
    quote! {
      #[inline(always)]
      pub fn #name(&self) -> ShaderPtrOf<T> {
        self.component::<#(#components)*>()
      }
    }
  } else {
    let output = format_ident!("Vec{}", len);
    let swizzle = format_ident!("swizzle{}", len);
    quote! {
      #[inline(always)]
      pub fn #name(&self) -> VectorSwizzlePtrView<#vec<T>, #output<T>> {
        self.#swizzle::<#(#components),*>()
      }
    }
  }
}

#[test]
fn all_swizzles_are_generated() {
  let code = impl_shader_swizzles_impl().to_string();
  let count = code.matches("pub fn").count();
  // for both the xyzw and rgba names: the value side has size^len methods for len in 1..=4, the
  // writable pointer has the size component references and the distinct swizzle views, the
  // readonly pointer has the size component references
  let expect: usize = [2_u8, 3, 4]
    .iter()
    .map(|size| {
      let value = (1..=4).map(|len| (*size as usize).pow(len)).sum::<usize>();
      let ptr = (1..=4)
        .flat_map(|len| all_components(*size, len))
        .filter(|c| is_distinct(c))
        .count();
      value + ptr + *size as usize
    })
    .sum();
  assert_eq!(count, expect * 2);
  // the distinct swizzle view count of vec4 is 4 + 12 + 24 + 24
  assert_eq!(
    (1..=4)
      .flat_map(|len| all_components(4, len))
      .filter(|c| is_distinct(c))
      .count(),
    64
  );
  assert!(code.contains("pub fn wzyx"));
  assert!(code.contains("pub fn abgr"));
  assert!(code.contains("swizzle4 :: < 3 , 2 , 1 , 0 >"));
}
