// pub fn only_accepct_struct(
//   input: syn::DeriveInput,
// ) -> Result<proc_macro2::TokenStream, syn::Error> {
//   match &input.data {
//     Data::Struct(_) => derive_struct(&input),
//     Data::Enum(e) => Err(syn::Error::new(
//       e.enum_token.span(),
//       "UniformBuffer implementations cannot be derived from enums",
//     )),
//     Data::Union(u) => Err(syn::Error::new(
//       u.union_token.span(),
//       "UniformBuffer implementations cannot be derived from unions",
//     )),
//   }
// }
