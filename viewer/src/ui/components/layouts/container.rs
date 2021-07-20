use rendiation_algebra::Vec4;

use crate::*;

pub struct Container<T> {
  pub size: Value<LayoutSize, T>,
  pub color: Vec4<f32>,
  position_computed: UIPosition,
}

impl<T> Container<T> {
    fn size(size: LayoutSize) -> Self {
       Self{
         size: Value::Static(size),
         color: Vec4::new(1., 1., 1., 0.),
         position_computed: Default::default(),
       }
    }
}

impl<T, C: Component<T>> ComponentAbility<T, C> for Container<T> {}

impl<T> Presentable for Container<T> {
  fn render(&self, builder: &mut PresentationBuilder) {
    builder.present.primitives.push(Primitive::Quad(Quad {
      x: 0.,
      y: 0.,
      width: 100.,
      height: 100.,
    }));
  }
}

impl<T> LayoutAble<T> for Container<T> {
  fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
    constraint.clamp(*self.size.get())
  }

  fn set_position(&mut self, position: UIPosition) {
    self.position_computed = position;
  }
}
