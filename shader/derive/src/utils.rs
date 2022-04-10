use syn::{punctuated::Punctuated, spanned::Spanned, Data, Field, Ident, Type, Visibility};

pub struct StructInfo {
  pub struct_name: Ident,
  pub fields_info: Vec<(Ident, Type)>,
  pub fields_raw: Vec<Field>,
}

impl StructInfo {
  pub fn new(input: &syn::DeriveInput) -> Self {
    let struct_name = input.ident.clone();
    let fields = only_named_struct_fields(input).unwrap();
    let fields_info = fields
      .iter()
      .map(|f| {
        let field_name = f.ident.as_ref().unwrap().clone();
        let ty = f.ty.clone();
        (field_name, ty)
      })
      .collect();

    let fields_raw = fields.iter().cloned().collect();

    StructInfo {
      struct_name,
      fields_info,
      fields_raw,
    }
  }

  pub fn _map_fields(
    &self,
    f: impl FnMut(&(Ident, Type)) -> proc_macro2::TokenStream,
  ) -> Vec<proc_macro2::TokenStream> {
    self.fields_info.iter().map(f).collect()
  }

  pub fn map_visible_fields(
    &self,
    f: impl FnMut((Ident, Type)) -> proc_macro2::TokenStream,
  ) -> Vec<proc_macro2::TokenStream> {
    self
      .fields_raw
      .iter()
      .filter_map(|f| {
        if matches!(f.vis, Visibility::Inherited) {
          None
        } else {
          let field_name = f.ident.as_ref().unwrap().clone();
          let ty = f.ty.clone();
          (field_name, ty).into()
        }
      })
      .map(f)
      .collect()
  }

  // pub fn map_fields_with_index(
  //   &self,
  //   f: impl FnMut((usize, &(Ident, Type))) -> proc_macro2::TokenStream,
  // ) -> Vec<proc_macro2::TokenStream> {
  //   self.fields_info.iter().enumerate().map(f).collect()
  // }
}

pub fn only_accept_struct(input: &syn::DeriveInput) -> Result<&syn::DeriveInput, syn::Error> {
  match &input.data {
    Data::Struct(_) => Ok(input),
    Data::Enum(e) => Err(syn::Error::new(
      e.enum_token.span(),
      "Cannot be derived from enums",
    )),
    Data::Union(u) => Err(syn::Error::new(
      u.union_token.span(),
      "Cannot be derived from unions",
    )),
  }
}

pub fn only_named_struct_fields(
  input: &syn::DeriveInput,
) -> Result<&Punctuated<Field, syn::token::Comma>, syn::Error> {
  only_accept_struct(input)?;
  let fields = if let syn::Data::Struct(syn::DataStruct {
    fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
    ..
  }) = input.data
  {
    named
  } else {
    return Err(syn::Error::new(
      input.span(),
      "Can only be derived from structs with named fields",
    ));
  };
  Ok(fields)
}
