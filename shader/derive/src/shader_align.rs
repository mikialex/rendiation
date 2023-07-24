use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{
  parse::Parser, AttrStyle, Attribute, Data, DeriveInput, Field, Fields, Type, Visibility,
};

fn get_ident_from_stream(tokens: TokenStream) -> Option<Ident> {
  match tokens.into_iter().next() {
    Some(TokenTree::Group(group)) => get_ident_from_stream(group.stream()),
    Some(TokenTree::Ident(ident)) => Some(ident),
    _ => None,
  }
}

/// get a simple #[foo(bar)] attribute, returning "bar"
fn get_simple_attr(attributes: &[Attribute], attr_name: &str) -> Option<Ident> {
  for attr in attributes {
    if let (AttrStyle::Outer, Some(outer_ident), Some(inner_ident)) = (
      &attr.style,
      attr.path.get_ident(),
      get_ident_from_stream(attr.tokens.clone()),
    ) {
      if *outer_ident == attr_name {
        return Some(inner_ident);
      }
    }
  }

  None
}

fn get_repr(attributes: &[Attribute]) -> Option<String> {
  get_simple_attr(attributes, "repr").map(|ident| ident.to_string())
}

fn check_attributes(attributes: &[Attribute]) -> Result<(), &'static str> {
  let repr = get_repr(attributes);
  match repr.as_deref() {
    Some("C") => Ok(()),
    Some("transparent") => Ok(()),
    _ => Err("Implementation requires the struct to be #[repr(C)] or #[repr(transparent)]"),
  }
}

pub fn shader_align_gen(
  mut input: DeriveInput,
  trait_name_str: &'static str,
  min_struct_alignment: usize,
) -> TokenStream {
  check_attributes(&input.attrs).unwrap();
  let trait_name = format_ident!("{}", trait_name_str);
  let trait_name = quote! {shadergraph::#trait_name};

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
        <#ty as #trait_name>::ALIGNMENT
    }
  };

  let field_alignments = fields.iter().map(|field| layout_alignment_of_ty(&field.ty));
  let struct_alignment = quote! {
      shadergraph::max_arr([
          #min_struct_alignment,
          #(#field_alignments,)*
      ])
  };

  // Generate names for each padding calculation function.
  let pad_fns: Vec<_> = (0..fields.len())
    .map(|index| format_ident!("_{}__{}Pad{}", input_name, trait_name_str, index))
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

  let pad_fn_impls: TokenStream = pad_fns
    .iter()
    .enumerate()
    .map(|(index, pad_fn)| {
      let starting_offset = offset_after_field(index);

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

              // We set our target alignment to the larger of the
              // alignment due to the previous field and the alignment
              // requirement of the next field.
              let alignment = #next_field_or_self_alignment;

              // Using everything we've got, compute our padding amount.
              shadergraph::align_offset(starting_offset, alignment)
          }
      }
    })
    .collect();

  let mut new_fields = fields.clone();
  new_fields.clear();
  fields.iter().enumerate().for_each(|(index, f)| {
    new_fields.push(f.clone());

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

      unsafe impl #impl_generics #trait_name for #input_name #ty_generics #where_clause {
          const ALIGNMENT: usize = #struct_alignment;
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
            let align = <Self as #trait_name>::ALIGNMENT;

            let zeroed: Self = shadergraph::Zeroable::zeroed();

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
