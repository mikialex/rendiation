pub fn build_graph() {
  let graph = Graph::new();
  let normal_pass = graph.pass("normal");
  let normal_target = graph.target("normal");

  let pass = graph.pass("scene").useQuad();
  RenderGraph::new().root().from_pass(pass)
}
