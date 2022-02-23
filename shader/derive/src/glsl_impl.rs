use glsl::{parser::Parse, syntax::*};
use quote::{format_ident, quote};
use std::collections::HashSet;

fn find_foreign_function(def: &mut FunctionDefinition) -> Vec<proc_macro2::TokenStream> {
  use glsl::visitor::*;

  // https://docs.rs/glsl/4.1.1/glsl/visitor/index.html
  struct ForeignFunctionCollector {
    depend_functions: HashSet<String>,
    exclude_functions: HashSet<String>,
  }

  impl Visitor for ForeignFunctionCollector {
    fn visit_expr(&mut self, exp: &Expr) -> Visit {
      if let Expr::FunCall(FunIdentifier::Identifier(ident), _) = exp {
        self.depend_functions.insert(ident.as_str().to_owned());
      }
      Visit::Children
    }
  }

  let mut collector = ForeignFunctionCollector {
    depend_functions: HashSet::new(),
    exclude_functions: vec![
      "vec2",
      "vec3",
      "vec4",
      "max",
      "min",
      "pow",
      "clamp",
      "mix",
      "length",
      "texture",
      "sampler2D",
      "smoothstep",
      "sin",
      "cos",
      "tan",
      "sqrt",
      "floor",
    ]
    .into_iter()
    .map(|s| s.to_owned())
    .collect(),
  };

  def.visit(&mut collector);

  collector
    .depend_functions
    .iter()
    .filter(|&f| !collector.exclude_functions.contains(f))
    .map(|f| {
      let prototype_name = gen_meta_name(f);
      quote! { &#prototype_name, }
    })
    .collect()
}

fn gen_meta_name(name: &str) -> syn::Ident {
  format_ident!("{}_SHADER_FUNCTION", name)
}

pub fn gen_glsl_function(glsl: &str) -> proc_macro2::TokenStream {
  let mut parsed = FunctionDefinition::parse(glsl).unwrap();
  let foreign = find_foreign_function(&mut parsed);

  let function_name = parsed.prototype.name.as_str();

  let prototype_name = gen_meta_name(function_name);
  let function_name = format_ident!("{}", function_name);
  let quoted_function_name = format!("{}", function_name);
  let quoted_source = glsl.to_string();
  let function_source = quote! {#quoted_source};

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionPrototype.html
  let return_type = convert_type(&parsed.prototype.ty.ty.ty);

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionParameterDeclarator.html
  let params: Vec<_> = parsed
    .prototype
    .parameters
    .iter()
    .map(|d| {
      if let FunctionParameterDeclaration::Named(_, p) = d {
        let ty = &p.ty;
        if ty.array_specifier.is_some() {
          panic!("unsupported") // todo improve
        }
        let name = p.ident.ident.as_str();
        (convert_type(&ty.ty), format_ident!("{}", name))
      } else {
        panic!("unsupported") // todo improve
      }
    })
    .collect();

  let (gen_function_inputs, input_node_prepare): (Vec<_>, Vec<_>) = params
    .iter()
    .map(|(ty, name)| {
      (
        quote! { #name: impl Into<Node<#ty>>, },
        quote! {
         let #name = #name.into().handle();
         parameters.push(#name);
        },
      )
    })
    .unzip();

  quote! {
    #[allow(non_upper_case_globals)]
    pub static #prototype_name: shadergraph::ShaderFunctionMetaInfo =
      shadergraph::ShaderFunctionMetaInfo{
        function_name: #quoted_function_name,
        function_source: #function_source,
        depend_functions:&[
          #(#foreign)*
        ]
      };

    pub fn #function_name (
      #(#gen_function_inputs)*
    ) -> shadergraph::Node<#return_type> {
      use shadergraph::*;

      let mut parameters = Vec::new();
      #(#input_node_prepare)*

      ShaderGraphNodeExpr::FunctionCall {
        meta: & #prototype_name,
        parameters,
      }.insert_graph()

    }

  }
}

fn convert_type(glsl: &TypeSpecifierNonArray) -> proc_macro2::TokenStream {
  let sampler_type = TypeName("sampler".to_owned());
  let texture_type = TypeName("texture2D".to_owned());

  {
    match glsl {
      TypeSpecifierNonArray::Float => quote! { f32 },
      TypeSpecifierNonArray::Vec2 => quote! { shadergraph::Vec2<f32> },
      TypeSpecifierNonArray::Vec3 => quote! { shadergraph::Vec3<f32> },
      TypeSpecifierNonArray::Vec4 => quote! { shadergraph::Vec4<f32> },
      TypeSpecifierNonArray::Mat4 => quote! { shadergraph::Mat4<f32> },
      TypeSpecifierNonArray::TypeName(ty) => {
        if ty == &sampler_type {
          quote! { shadergraph::ShaderSampler }
        } else if ty == &texture_type {
          quote! { shadergraph::ShaderTexture }
        } else {
          panic!("unsupported param type {:?}", glsl)
        }
      }
      _ => panic!("unsupported param type {:?}", glsl),
    }
  }
}
