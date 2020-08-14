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
    exclude_functions: vec!["vec2", "vec3", "vec4", "max", "min", "pow", "clamp", "mix"]
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
      quote! { .declare_function_dep(#prototype_name.clone()) }
    })
    .collect()
}

pub fn gen_glsl_function(glsl: &str) -> proc_macro2::TokenStream {
  let glsl = glsl.trim_start();
  let mut parsed = syntax::FunctionDefinition::parse(glsl).unwrap();
  let foreign = find_foreign_function(&mut parsed);

  let function_name = parsed.prototype.name.as_str();

  let prototype_name = format_ident!("{}_FUNCTION", function_name);
  let function_name = format_ident!("{}", function_name);
  let quoted_function_name = format!("{}", function_name);
  let quoted_source = format!("{}", glsl);

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

  let (gen_function_inputs, gen_node_connect): (Vec<_>, Vec<_>) = params
    .iter()
    .map(|(ty, name)| {
      (
        quote! { #name: rendiation_shadergraph::ShaderGraphNodeHandle<#ty>, },
        quote! { graph.nodes.connect_node(#name.cast_type(), result); },
      )
    })
    .unzip();

  // we cant use lazy_static in marco so let's try once_cell
  quote! {
    pub static #prototype_name: once_cell::sync::Lazy<
    std::sync::Arc<
      rendiation_shadergraph::ShaderFunction
    >> =
    once_cell::sync::Lazy::new(||{
      std::sync::Arc::new(
        rendiation_shadergraph::ShaderFunction::new(
          #quoted_function_name,
          Some(#quoted_source)
        )
        #(#foreign)*
      )
    });

    pub fn #function_name (
      #(#gen_function_inputs)*
    ) -> rendiation_shadergraph::ShaderGraphNodeHandle<#return_type> {
      use rendiation_shadergraph::*;

      let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
      let graph = guard.as_mut().unwrap();
      let result = graph
        .nodes
        .create_node(ShaderGraphNode::<#return_type>::new(
            ShaderGraphNodeData::Function(
              FunctionNode {
                prototype: #prototype_name.clone()
              },
            )
          ).to_any()
        );
      unsafe {
        #(#gen_node_connect)*
        result.cast_type()
      }
    }

  }
}

fn convert_type(glsl: &syntax::TypeSpecifierNonArray) -> proc_macro2::TokenStream {
  use syntax::TypeSpecifierNonArray::*;
  match glsl {
    Float => quote! { f32 },
    Vec2 => quote! { rendiation_math::Vec2<f32> },
    Vec3 => quote! { rendiation_math::Vec3<f32> },
    Vec4 => quote! { rendiation_math::Vec4<f32> },
    Mat4 => quote! { rendiation_math::Mat4<f32> },
    _ => panic!("unsupported param type"),
  }
}
