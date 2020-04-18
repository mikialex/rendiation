

pub struct TargetNode {
  name: String,
  from_pass_id: Vec<usize>,
  to_pass_id: Vec<usize>,
}

impl TargetNode {
  pub fn new() -> Self {
    todo!();
  }

  pub fn from(&mut self, node: PassNode) -> &mut Self {
    self
  }
}


pub struct PassNode {
  name: String,
  from_target_id: Vec<usize>,
  to_target_id: Vec<usize>,
}

impl PassNode {
  pub fn new() -> Self {
    todo!();
  }

  pub fn draw() {}
}