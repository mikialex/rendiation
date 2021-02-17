use glsl::parser::Parse;
use glsl::syntax;
use quote::{format_ident, quote};
use std::collections::HashSet;

fn find_foreign_function(def: &mut syntax::FunctionDefinition) -> Vec<proc_macro2::TokenStream> {
  use glsl::syntax::*;
  use glsl::visitor::*;

  // https://docs.rs/glsl/4.1.1/glsl/visitor/index.html
  struct ForeignFunctionCollector {
    depend_functions: HashSet<String>,
    exclude_functions: HashSet<String>,
  }

  impl Visitor for ForeignFunctionCollector {
    fn visit_expr(&mut self, exp: &mut Expr) -> Visit {
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
      let prototype_name = format_ident!("{}_FUNCTION", f);
      quote! { .declare_function_dep(& #prototype_name) }
    })
    .collect()
}

pub fn gen_glsl_function(
  glsl: &str,
  as_inner: bool,
  override_name: &str,
) -> proc_macro2::TokenStream {
  let glsl = glsl.trim_start();
  let mut parsed = syntax::FunctionDefinition::parse(glsl).unwrap();
  let foreign = find_foreign_function(&mut parsed);

  let function_name = parsed.prototype.name.as_str();

  let prototype_name = format_ident!("{}_FUNCTION", function_name);
  let function_name = format_ident!("{}", function_name);
  let quoted_function_name = if as_inner {
    override_name.to_owned()
  } else {
    format!("{}", function_name)
  };
  let quoted_source = format!("{}", glsl);
  let function_source = if as_inner {
    quote! { None }
  } else {
    quote! {Some(#quoted_source)}
  };

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionPrototype.html
  let return_type = convert_type(&parsed.prototype.ty.ty.ty);

  // https://docs.rs/glsl/4.1.1/glsl/syntax/struct.FunctionParameterDeclarator.html
  let params: Vec<_> = parsed
    .prototype
    .parameters
    .iter()
    .map(|d| {
      if let syntax::FunctionParameterDeclaration::Named(_, p) = d {
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

  let input_node_prepare: Vec<_> = params
    .iter()
    .map(|(_, name)| {
      quote! { let #name = #name.to_node(graph).handle.cast_type(); }
    })
    .collect();

  let (gen_function_inputs, gen_node_connect): (Vec<_>, Vec<_>) = params
    .iter()
    .map(|(ty, name)| {
      (
        quote! { #name: impl rendiation_shadergraph::ShaderGraphNodeOrConst<Output = #ty>, },
        quote! { graph.nodes.connect_node(#name, result); },
      )
    })
    .unzip();

  quote! {
    pub static #prototype_name: once_cell::sync::Lazy<rendiation_shadergraph::ShaderFunctionMetaInfo> =
    once_cell::sync::Lazy::new(|| {
        rendiation_shadergraph::ShaderFunctionMetaInfo::new(
          #quoted_function_name,
          #function_source
        )
        #(#foreign)*
    });

    pub fn #function_name (
      #(#gen_function_inputs)*
    ) -> rendiation_shadergraph::Node<#return_type> {
      use rendiation_shadergraph::*;

      modify_graph(|graph| {
        let node = ShaderGraphNode::<#return_type>::new(
          ShaderGraphNodeData::Function(
            FunctionNode {
              prototype: & #prototype_name
            },
          )
        );
        let result = graph.insert_node(node).handle;
        unsafe {
          #(#input_node_prepare)*
          #(#gen_node_connect)*
          result.cast_type().into()
        }
      })

    }

  }
}

fn convert_type(glsl: &syntax::TypeSpecifierNonArray) -> proc_macro2::TokenStream {
  use syntax::TypeSpecifierNonArray::*;
  let sampler_type = glsl::syntax::TypeName("sampler".to_owned());
  let texture_type = glsl::syntax::TypeName("texture2D".to_owned());
  match glsl {
    Float => quote! { f32 },
    Vec2 => quote! { rendiation_algebra::Vec2<f32> },
    Vec3 => quote! { rendiation_algebra::Vec3<f32> },
    Vec4 => quote! { rendiation_algebra::Vec4<f32> },
    Mat4 => quote! { rendiation_algebra::Mat4<f32> },
    TypeName(ty) => {
      if ty == &sampler_type {
        quote! { rendiation_shadergraph::ShaderSampler }
      } else if ty == &texture_type {
        quote! { rendiation_shadergraph::ShaderTexture }
      } else {
        panic!("unsupported param type {:?}", glsl)
      }
    }
    _ => panic!("unsupported param type {:?}", glsl),
  }
}
