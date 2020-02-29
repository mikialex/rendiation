

pub struct RenderGraph {
    pass_nodes: Vec<PassNode>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            pass_nodes: Vec::new()
        }
    }

    pub fn pass() -> PassNode {
        PassNode {}
    }
}

pub struct PassNode {

}

impl PassNode {
    pub fn from(&mut self, node: TargetNode) -> &mut Self {
        self
    }
}

pub struct TargetNode {

}