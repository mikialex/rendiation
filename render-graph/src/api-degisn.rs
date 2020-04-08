pub fn build_graph(){
    let normal_pass = pass("normal");
    let normal_target = target("normal");

    let pass = pass("scene").useQuad();
    RenderGraph::new()
    .root().from_pass(pass)
}