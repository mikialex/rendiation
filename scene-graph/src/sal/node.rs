use std::any::Any;

pub trait ShaderGraphNode: Sized{
    type NodeType: NodeType;
}

pub trait NodeType: Any + Sized{}

impl NodeType for (){}

pub trait InputType: Any + Sized{
    type InputTypeOne: NodeType;
}
impl InputType for (){
    type InputTypeOne = ();
}
