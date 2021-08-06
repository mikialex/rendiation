use crate::*;

impl<T> ComponentArray<Child<T>> {
  pub fn flex_group() -> Self {
    Self {
      children: Vec::new(),
    }
  }

  pub fn add_fixed_child(
    mut self,
    child: impl UIComponent<T>,
    alignment: Option<CrossAxisAlignment>,
  ) -> Self {
    self.children.push(Child::Fixed {
      widget: Box::new(child),
      result: Default::default(),
      position: Default::default(),
      alignment,
    });
    self
  }

  pub fn add_flex_child(
    mut self,
    child: impl UIComponent<T>,
    flex: f32,
    alignment: Option<CrossAxisAlignment>,
  ) -> Self {
    self.children.push(Child::Flex {
      widget: Box::new(child),
      flex,
      result: Default::default(),
      position: Default::default(),
      alignment,
    });
    self
  }
}
