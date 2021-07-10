pub enum Value<T, U> {
  Static(T),
  Dynamic(DynamicValue<T, U>),
}
impl<T, U> Value<T, U> {
  pub fn update(&mut self, ctx: &U) -> &T {
    todo!()
  }
}

pub struct DynamicValue<T, U> {
  fun: Box<dyn Fn(&U) -> T>,
  value: Option<T>,
}
