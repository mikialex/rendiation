use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parser, parse_quote, Data, DeriveInput, Field, Fields, Type, Visibility};

pub fn std140_impl(mut input: DeriveInput) -> TokenStream {
  let input_name = &input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  //  We could potentially
  // support transparent tuple structs in the future.
  let fields = match &mut input.data {
    Data::Struct(data) => match &mut data.fields {
      Fields::Named(fields) => &mut fields.named,
      Fields::Unnamed(_) => panic!("Tuple structs are not supported"),
      Fields::Unit => panic!("Unit structs are not supported"),
    },
    Data::Enum(_) | Data::Union(_) => panic!("Only structs are supported"),
  };

  fields.iter().for_each(|f| {
    if matches!(f.vis, Visibility::Inherited) {
      panic!("private field not allowed")
    }
  });

  // Gives an expression returning the layout-specific alignment for the type.
  let layout_alignment_of_ty = |ty: &Type| {
    quote! {
        <#ty as shadergraph::Std140>::ALIGNMENT
    }
  };

  // Gives an expression telling whether the type should have trailing padding
  // at least equal to its alignment.
  let layout_pad_at_end_of_ty = |ty: &Type| {
    quote! {
        <#ty as shadergraph::Std140>::PAD_AT_END
    }
  };

  let field_alignments = fields.iter().map(|field| layout_alignment_of_ty(&field.ty));
  let struct_alignment = quote! {
      shadergraph::max_arr([
          16,
          #(#field_alignments,)*
      ])
  };

  // Generate names for each padding calculation function.
  let pad_fns: Vec<_> = (0..fields.len())
    .map(|index| format_ident!("_{}__{}Pad{}", input_name, "std140", index))
    .collect();

  // Computes the offset immediately AFTER the field with the given index.
  //
  // This function depends on the generated padding calculation functions to
  // do correct alignment. Be careful not to cause recursion!
  let offset_after_field = |target: usize| {
    let mut output = vec![quote!(0usize)];

    for index in 0..=target {
      let field_ty = &fields[index].ty;

      output.push(quote! {
          + ::core::mem::size_of::<#field_ty>()
      });

      // For every field except our target field, also add the generated
      // padding. Padding occurs after each field, so it isn't included in
      // this value.
      if index < target {
        let pad_fn = &pad_fns[index];
        output.push(quote! {
            + #pad_fn()
        });
      }
    }

    output.into_iter().collect::<TokenStream>()
  };

  let pad_fn_impls: TokenStream = fields
    .iter()
    .enumerate()
    .map(|(index, prev_field)| {
      let pad_fn = &pad_fns[index];

      let starting_offset = offset_after_field(index);
      let prev_field_has_end_padding = layout_pad_at_end_of_ty(&prev_field.ty);
      let prev_field_alignment = layout_alignment_of_ty(&prev_field.ty);

      let next_field_or_self_alignment = if index + 1 == fields.len() {
        quote!(#struct_alignment)
      } else {
        layout_alignment_of_ty(&fields[index + 1].ty)
      };

      quote! {
          /// Tells how many bytes of padding have to be inserted after
          /// the field with index #index.
          #[allow(non_snake_case)]
          const fn #pad_fn() -> usize {
              // First up, calculate our offset into the struct so far.
              // We'll use this value to figure out how far out of
              // alignment we are.
              let starting_offset = #starting_offset;

              // If the previous field is a struct or array, we must align
              // the next field to at least THAT field's alignment.
              let min_alignment = if #prev_field_has_end_padding {
                  #prev_field_alignment
              } else {
                  0
              };

              // We set our target alignment to the larger of the
              // alignment due to the previous field and the alignment
              // requirement of the next field.
              let alignment = shadergraph::max(
                  #next_field_or_self_alignment,
                  min_alignment,
              );

              // Using everything we've got, compute our padding amount.
              shadergraph::align_offset(starting_offset, alignment)
          }
      }
    })
    .collect();

  let mut new_fields = fields.clone();
  new_fields.clear();
  fields.iter().enumerate().for_each(|(index, f)| {
    let mut f = f.clone();
    let ty = &f.ty;
    f.ty = parse_quote!(<#ty as shadergraph::Std140TypeMapper>::StorageType);
    new_fields.push(f);

    let pad_field_name = format_ident!("_pad{}", index);
    let pad_fn = &pad_fns[index];
    let pad_field = Field::parse_named
      .parse2(quote! { #pad_field_name: [u8; #pad_fn()] })
      .unwrap();
    new_fields.push(pad_field);
  });
  *fields = new_fields.clone();

  let trait_impl = quote! {
      #pad_fn_impls

      unsafe impl #impl_generics shadergraph::Zeroable for #input_name #ty_generics #where_clause {}
      unsafe impl #impl_generics shadergraph::Pod for #input_name #ty_generics #where_clause {}

      unsafe impl #impl_generics shadergraph::Std140 for #input_name #ty_generics #where_clause {
          const ALIGNMENT: usize = #struct_alignment;
          const PAD_AT_END: bool = true;
      }
  };

  let debug_fields: TokenStream = fields
    .iter()
    .map(|field| {
      let field_name = field.ident.as_ref().unwrap();
      let field_ty = &field.ty;

      quote! {
          fields.push(Field {
              name: stringify!(#field_name),
              size: ::core::mem::size_of::<#field_ty>(),
              offset: (&zeroed.#field_name as *const _ as usize)
                  - (&zeroed as *const _ as usize),
          });
      }
    })
    .collect();

  let debug = quote! {
    impl #impl_generics #input_name #ty_generics #where_clause {
        fn debug_metrics() -> String {
            let size = ::core::mem::size_of::<Self>();
            let align = <Self as shadergraph::Std140>::ALIGNMENT;

            let zeroed: Self = ::crevice::internal::bytemuck::Zeroable::zeroed();

            #[derive(Debug)]
            struct Field {
                name: &'static str,
                offset: usize,
                size: usize,
            }
            let mut fields = Vec::new();

            #debug_fields

            format!("Size {}, Align {}, fields: {:#?}", size, align, fields)
        }

        fn debug_definitions() -> &'static str {
            stringify!(
                #new_fields
                #pad_fn_impls
            )
        }
    }
  };

  quote! {
    #input
    #trait_impl
    #debug
  }
}
