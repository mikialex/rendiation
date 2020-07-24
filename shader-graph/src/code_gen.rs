use crate::code_builder::*;
use crate::*;
use std::collections::HashMap;
use arena_graph::*;

struct CodeGenCtx {
  var_guid: usize,
  code_gen_history: HashMap<ShaderGraphNodeHandleUntyped, MiddleVariableCodeGenResult>,
  depend_functions: HashSet<Arc<ShaderFunction>>,
}

impl CodeGenCtx {
  fn new() -> Self {
    Self {
      var_guid: 0,
      code_gen_history: HashMap::new(),
      depend_functions: HashSet::new(),
    }
  }

  fn add_node_result(&mut self, mut result: MiddleVariableCodeGenResult) -> &str {
    result.var_name = format!("{}", self.var_guid);
    self.var_guid += 1;
    self
      .code_gen_history
      .entry(result.ref_node)
      .or_insert(result)
      .expression_str
      .as_str()
  }

  fn add_fn_dep(&mut self, node: &FunctionNode) {
    self.depend_functions.insert(node.prototype.clone());
  }

  fn gen_fn_depends(&self, builder: &mut CodeBuilder) {
    // let mut resolved_fn: HashSet<Arc<ShaderFunction>> = HashSet::new();
    // self.depend_functions.iter().for_each(|f|{
    //   if f.depend_functions.len() > 0{
    //     let root = fn_dep_graph.create_node(f.clone());
    //     let mut fn_dep_graph: ArenaGraph<Arc<ShaderFunction>> = ArenaGraph::new();
    //     let mut resolving_fn: HashMap<Arc<ShaderFunction>, ArenaGraphNodeHandle<Arc<ShaderFunction>>> = HashSet::new();



    //     fn push_node(
    //       n: ArenaGraphNodeHandle<Arc<ShaderFunction>>, 
    //       g: &mut ArenaGraph<Arc<ShaderFunction>>,
    //       resolving_fn: HashMap<Arc<ShaderFunction>, ArenaGraphNodeHandle<Arc<ShaderFunction>>>
    //     ){
    //       let node = g.get_node(n);
    //       let node_f = node.data();

    //       node_f.depend_functions.iter().for_each(|f_d|{
    //         if !resolving_fn.contains_key(&f_d){
    //           let dep_n = fn_dep_graph.create_node(f_d.clone());
    //           g.connect_node(dep_n, node);
    //           resolving_fn.insert(node, dep_n)
    //         } else{

    //         }
    //       });
          
    //       node.from().iter().for_each(|from|{
    //         push_node(n, g)
    //       })
    //     }

    //     push_node(root, &mut fn_dep_graph)
    //   }
    // });
    todo!()

  }
}

struct MiddleVariableCodeGenResult {
  ref_node: ShaderGraphNodeHandleUntyped,
  var_name: String,
  expression_str: String,
}

impl MiddleVariableCodeGenResult {
  fn new(ref_node: ShaderGraphNodeHandleUntyped, expression_str: String) -> Self {
    Self {
      ref_node,
      var_name: String::new(), // this will initialize in code gen ctx
      expression_str,
    }
  }
}

impl ShaderGraph {
  fn gen_code_node(
    &self,
    handle: ShaderGraphNodeHandleUntyped,
    ctx: &mut CodeGenCtx,
    builder: &mut CodeBuilder,
  ) {
    builder.write_ln("");

    self
      .nodes
      .topological_order_list(handle)
      .unwrap()
      .iter()
      .for_each(|&h| {
        // this node has generated, skip
        if ctx.code_gen_history.contains_key(&h) {
          return;
        }

        let node_wrap = self.nodes.get_node(h);
        use ShaderGraphNodeData::*;
        let result = match &node_wrap.data().data {
          Function(n) => {
            ctx.add_fn_dep(n);
            ctx.add_node_result(MiddleVariableCodeGenResult::new(
              h,
              node_wrap
                .from()
                .iter()
                .map(|from| ctx.code_gen_history.get(from).unwrap().var_name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            ))
          }
          Input(node) => {
            ctx.add_node_result(MiddleVariableCodeGenResult::new(h, node.name.clone()))
          }
        };
        builder.write_ln(result);
      });
  }

  pub fn gen_code_vertex(&self) -> String {
    let mut ctx = CodeGenCtx::new();
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    self.varyings.iter().for_each(|&v| {
      self.gen_code_node(v, &mut ctx, &mut builder);
    });

    self.gen_code_node(
      self.vertex_position.expect("vertex position not set"),
      &mut ctx,
      &mut builder,
    );

    builder.write_ln("").un_tab().write_ln("}");
    builder.output()
  }

  pub fn gen_code_frag(&self) -> String {
    let mut ctx = CodeGenCtx::new();
    let mut builder = CodeBuilder::new();
    builder.write_ln("void main() {").tab();

    self.frag_outputs.iter().for_each(|&v| {
      self.gen_code_node(v, &mut ctx, &mut builder);
    });

    builder.write_ln("").un_tab().write_ln("}");
    builder.output()
  }
}
