use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Visibility, parse::Parser, spanned::Spanned};

use crate::shader_struct::derive_shader_struct;
use crate::utils::StructInfo;

/// Only `#[repr(C)]` is accepted. Other repr hints like `align` or `packed` change the rust
/// layout, the generated layout assertions will reject them anyway, but rejecting them here
/// gives a better error message.
fn check_repr(input: &DeriveInput) -> syn::Result<()> {
  let mut has_repr_c = false;
  for attr in &input.attrs {
    if !attr.path().is_ident("repr") {
      continue;
    }
    attr.parse_nested_meta(|meta| {
      if meta.path.is_ident("C") {
        has_repr_c = true;
        Ok(())
      } else {
        Err(meta.error("host shareable shader struct only supports #[repr(C)]"))
      }
    })?;
  }

  if has_repr_c {
    Ok(())
  } else {
    Err(syn::Error::new(
      input.ident.span(),
      "host shareable shader struct requires #[repr(C)]",
    ))
  }
}

pub fn shader_align_gen(
  input: DeriveInput,
  trait_name_str: &'static str,
  min_struct_alignment: usize,
) -> TokenStream {
  shader_align_gen_impl(input, trait_name_str, min_struct_alignment)
    .unwrap_or_else(syn::Error::into_compile_error)
}

/// The generated code inserts explicit padding fields so the rust repr(C) layout matches the
/// shader side layout, and then asserts at compile time that every field offset, the struct size
/// and the struct alignment are exactly what we expected. This makes sure no implicit rust
/// padding exists(which is required by the Pod impl), and the layout never silently mismatches.
fn shader_align_gen_impl(
  mut input: DeriveInput,
  trait_name_str: &'static str,
  min_struct_alignment: usize,
) -> syn::Result<TokenStream> {
  check_repr(&input)?;
  if !input.generics.params.is_empty() {
    return Err(syn::Error::new(
      input.generics.span(),
      "generic struct is not supported by host shareable shader struct",
    ));
  }

  let trait_ident = format_ident!("{}", trait_name_str);
  let trait_name = quote! {rendiation_shader_api::#trait_ident};
  let input_name = input.ident.clone();

  let fields = match &mut input.data {
    Data::Struct(data) => match &mut data.fields {
      Fields::Named(fields) => &mut fields.named,
      other => {
        return Err(syn::Error::new(
          other.span(),
          "only struct with named fields is supported by host shareable shader struct",
        ));
      }
    },
    _ => {
      return Err(syn::Error::new(
        input_name.span(),
        "only struct is supported by host shareable shader struct",
      ));
    }
  };

  if fields.is_empty() {
    return Err(syn::Error::new(
      input_name.span(),
      "empty struct is not supported by host shareable shader struct",
    ));
  }

  if let Some(f) = fields
    .iter()
    .find(|f| matches!(f.vis, Visibility::Inherited))
  {
    return Err(syn::Error::new(
      f.span(),
      "private field is not allowed, private fields are reserved for the generated paddings",
    ));
  }

  let field_names: Vec<_> = fields.iter().map(|f| f.ident.clone().unwrap()).collect();
  let field_tys: Vec<_> = fields.iter().map(|f| f.ty.clone()).collect();
  let field_count = fields.len();

  let prefix = format!("__{input_name}_{trait_name_str}");
  let align_const = format_ident!("{prefix}_ALIGN");
  let size_const = format_ident!("{prefix}_SIZE");
  let offset_consts: Vec<_> = (0..field_count)
    .map(|i| format_ident!("{prefix}_OFFSET_{i}"))
    .collect();
  let pad_consts: Vec<_> = (0..field_count)
    .map(|i| format_ident!("{prefix}_PAD_{i}"))
    .collect();

  // Every padding and offset is a separate const item and each one only depends on the previous
  // one, const items are evaluated only once, so the evaluation cost is linear to the field
  // count. (Using const fn here is exponential because const fn calls are not memoized)
  let layout_consts = (0..field_count).map(|i| {
    let offset_const = &offset_consts[i];
    let pad_const = &pad_consts[i];
    let ty = &field_tys[i];

    let offset = if i == 0 {
      quote! { 0 }
    } else {
      let prev_offset = &offset_consts[i - 1];
      let prev_ty = &field_tys[i - 1];
      let prev_pad = &pad_consts[i - 1];
      quote! { #prev_offset + ::core::mem::size_of::<#prev_ty>() + #prev_pad }
    };

    let next_alignment = if i + 1 == field_count {
      quote! { #align_const }
    } else {
      let next_ty = &field_tys[i + 1];
      quote! { <#next_ty as #trait_name>::ALIGNMENT }
    };

    quote! {
      #[doc(hidden)]
      #[allow(non_upper_case_globals)]
      const #offset_const: usize = #offset;
      #[doc(hidden)]
      #[allow(non_upper_case_globals)]
      const #pad_const: usize = rendiation_shader_api::align_offset(
        #offset_const + ::core::mem::size_of::<#ty>(),
        #next_alignment,
      );
    }
  });

  let last_offset = &offset_consts[field_count - 1];
  let last_ty = &field_tys[field_count - 1];
  let last_pad = &pad_consts[field_count - 1];

  let layout_consts = quote! {
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    const #align_const: usize = rendiation_shader_api::max_arr([
      #min_struct_alignment,
      #(<#field_tys as #trait_name>::ALIGNMENT,)*
    ]);
    #(#layout_consts)*
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    const #size_const: usize = #last_offset + ::core::mem::size_of::<#last_ty>() + #last_pad;
  };

  let layout_assertions = quote! {
    const _: () = {
      #(
        assert!(
          ::core::mem::offset_of!(#input_name, #field_names) == #offset_consts,
          concat!(
            "shader layout mismatch: unexpected offset of field `",
            stringify!(#field_names), "` in `", stringify!(#input_name), "`"
          )
        );
      )*
      assert!(
        ::core::mem::size_of::<#input_name>() == #size_const,
        concat!(
          "shader layout mismatch: unexpected size of `", stringify!(#input_name),
          "`, the rust layout contains implicit padding"
        )
      );
      assert!(
        ::core::mem::align_of::<#input_name>() <= #align_const,
        concat!(
          "shader layout mismatch: the rust alignment of `", stringify!(#input_name),
          "` is larger than the shader alignment"
        )
      );
    };
  };

  let mut new_fields = fields.clone();
  new_fields.clear();
  for (index, f) in fields.iter().enumerate() {
    new_fields.push(f.clone());

    let pad_field_name = format_ident!("_pad{}", index);
    let pad_const = &pad_consts[index];
    let pad_field = Field::parse_named.parse2(quote! { #pad_field_name: [u8; #pad_const] })?;
    new_fields.push(pad_field);
  }
  *fields = new_fields;

  // the padding fields are private, so they are not visible to the shader struct
  let shader_struct = derive_shader_struct(&StructInfo::new(&input), Some(trait_ident));

  Ok(quote! {
    #input
    #layout_consts
    #layout_assertions

    unsafe impl rendiation_shader_api::Zeroable for #input_name {}
    unsafe impl rendiation_shader_api::Pod for #input_name {}

    unsafe impl #trait_name for #input_name {
      const ALIGNMENT: usize = #align_const;
    }

    #shader_struct
  })
}
