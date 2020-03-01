
pub struct RenderGraph {
    nodes: Vec<RenderGraphNode>,
    sorted: Vec<usize>
}

impl RenderGraph {
    pub fn new() -> Self {
        let root = TargetNode::new();
        let mut graph = Self {
            nodes: Vec::new(),
            sorted: Vec::new(),
        };
        graph.nodes[0] = RenderGraphNode::Target(root);
        graph
    }

    pub fn get_root(&mut self) -> &mut TargetNode {
        if let RenderGraphNode::Target(target) = &mut self.nodes[0] {
            target
        } else {
            panic!();
        }
    }

    // pub fn pass(&mut self) -> &mut PassNode {
    //     // let node = PassNode::new();
    //     // node
    // }

}

pub enum RenderGraphNode {
    Pass(PassNode),
    Target(TargetNode)
}

impl RenderGraphNode{
    // pub fn 
}

pub struct PassNode {
    name: String,
    from_target_id: Vec<usize>,
    to_target_id: Vec<usize>
}

impl PassNode {
    pub fn new() -> Self {
        todo!();
    }
}

pub struct TargetNode {
    name: String,
    from_pass_id: Vec<usize>,
    to_pass_id: Vec<usize>
}

impl TargetNode {
    pub fn new() -> Self {
        todo!();
    }

    pub fn from(&mut self, node: TargetNode) -> &mut Self {
        self
    }
}