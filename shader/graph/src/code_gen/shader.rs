// use crate::*;

// impl ShaderGraphBindGroup {
//   pub fn gen_header(
//     &self,
//     builder: &ShaderGraphShaderBuilder,
//     index: usize,
//     stage: ShaderStages,
//   ) -> String {
//     self
//       .inputs
//       .iter()
//       .enumerate()
//       .filter_map(|(i, h)| {
//         if stage != h.1 {
//           return None;
//         }
//         match &h.0 {
//           ShaderGraphBindgroupEntry::NoneUBO(node) => Some(format!(
//             "layout(set = {}, binding = {}) uniform {} {};\n",
//             index,
//             i,
//             builder.type_id_map.get(&info.node_type).unwrap(),
//             input.name.as_str()
//           )),
//           ShaderGraphBindgroupEntry::UBO((info, _)) => Some(format!(
//             "layout(set = {}, binding = {}) {};",
//             index, i, info.code_cache
//           )),
//         }
//       })
//       .collect::<Vec<_>>()
//       .join("\n")
//   }
// }

// impl ShaderGraphShaderBuilder {
//   pub(super) fn gen_header_vert(&self) -> String {
//     let mut result = String::from("#version 450\n");

//     // attributes
//     result += self
//       .attributes
//       .iter()
//       .map(|a| {
//         let info = self.nodes.get_node(a.0.handle()).data();
//         let input = info.unwrap_as_input();
//         format!(
//           "layout(location = {}) in {} {};",
//           a.1,
//           self.type_id_map.get(&info.node_type).unwrap(),
//           input.name.as_str()
//         )
//       })
//       .collect::<Vec<String>>()
//       .join("\n")
//       .as_ref();

//     result += "\n";

//     // varyings
//     result += self
//       .varyings
//       .iter()
//       .map(|a| {
//         let info = self.nodes.get_node(a.0.handle()).data();
//         // let id = info.unwrap_as_vary();
//         format!(
//           "layout(location = {}) out {} {};",
//           a.1,
//           self.type_id_map.get(&info.node_type).unwrap(),
//           format!("vary{}", a.1)
//         )
//       })
//       .collect::<Vec<_>>()
//       .join("\n")
//       .as_ref();

//     result += "\n";

//     result += self.gen_bindgroups_header(ShaderStages::Vertex).as_str();

//     result
//   }

//   pub(super) fn gen_bindgroups_header(&self, stage: ShaderStages) -> String {
//     self
//       .bindgroups
//       .iter()
//       .enumerate()
//       .map(|(i, b)| b.gen_header(self, i, stage))
//       .collect::<Vec<_>>()
//       .join("\n")
//   }

//   pub(super) fn gen_header_frag(&self) -> String {
//     let mut result = String::from("#version 450\n");

//     result += self.gen_bindgroups_header(ShaderStages::Fragment).as_str();

//     // varyings
//     result += self
//       .varyings
//       .iter()
//       .map(|a| {
//         let info = self.nodes.get_node(a.0.handle()).data();
//         // let id = info.unwrap_as_vary();
//         format!(
//           "layout(location = {}) in {} {};",
//           a.1,
//           self.type_id_map.get(&info.node_type).unwrap(),
//           format!("vary{}", a.1)
//         )
//       })
//       .collect::<Vec<_>>()
//       .join("\n")
//       .as_ref();

//     result += "\n";

//     result += self
//       .frag_outputs
//       .iter()
//       .map(|(_, index)| format!("layout(location = {}) out vec4 frag{};", index, index))
//       .collect::<Vec<_>>()
//       .join("\n")
//       .as_ref();

//     result
//   }
// }

// impl ShaderGraphShaderBuilder {
//   fn gen_code_node(
//     &self,
//     handle: ShaderGraphNodeRawHandleUntyped,
//     ctx: &mut CodeGenScopeCtx,
//     builder: &mut CodeBuilder,
//   ) {
//     builder.write_ln("");

//     let depends = self.nodes.topological_order_list(handle).unwrap();

//     depends.iter().for_each(|&h| {
//       // this node has generated, skip
//       if ctx.code_gen_history.contains_key(&h) {
//         return;
//       }

//       let node_wrap = self.nodes.get_node(h);

//       // None is input node, skip
//       if let Some(result) = node_wrap.data().gen_node_record(h, self, ctx) {
//         builder.write_ln(&format!("{}", result));
//         ctx.add_node_result(result);
//       }
//     });
//   }

//   pub fn gen_code_vertex(&self) -> String {
//     let mut ctx = CodeGenScopeCtx::new(0);
//     let mut builder = CodeBuilder::default();
//     builder.write_ln("void main() {").tab();

//     self.varyings.iter().for_each(|(v, _)| {
//       self.gen_code_node(v.handle(), &mut ctx, &mut builder);
//     });

//     self.gen_code_node(
//       unsafe {
//         self
//           .vertex_position
//           .as_ref()
//           .expect("vertex position not set")
//           .handle()
//           .cast_type()
//       },
//       &mut ctx,
//       &mut builder,
//     );

//     builder.write_ln("").un_tab().write_ln("}");

//     let header = self.gen_header_vert();
//     let main = builder.output();
//     let lib = ctx.gen_fn_depends();
//     header + "\n" + &lib + "\n" + &main
//   }

//   pub fn gen_code_frag(&self) -> String {
//     let mut ctx = CodeGenScopeCtx::new(0);
//     let mut builder = CodeBuilder::default();
//     builder.write_ln("void main() {").tab();

//     self.frag_outputs.iter().for_each(|(v, _)| {
//       self.gen_code_node(v.handle(), &mut ctx, &mut builder);
//     });

//     builder.write_ln("").un_tab().write_ln("}");

//     let header = self.gen_header_frag();
//     let main = builder.output();
//     let lib = ctx.gen_fn_depends();
//     header + "\n" + &lib + "\n" + &main
//   }
// }
