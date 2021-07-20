pub enum Value<T, U> {
  Static(T),
  Dynamic(DynamicValue<T, U>),
}

impl<T, U> From<T> for Value<T, U> {
  fn from(v: T) -> Self {
    Self::Static(v)
  }
}

impl<T, U> Value<T, U> {
  pub fn update(&mut self, ctx: &U) -> &T {
    match self {
      Value::Static(v) => v,
      Value::Dynamic(d) => {
        d.value = (d.fun)(ctx);
        &d.value
      }
    }
  }

  pub fn get(&self) -> &T {
    match self {
      Value::Static(v) => v,
      Value::Dynamic(d) => &d.value,
    }
  }
}

pub struct DynamicValue<T, U> {
  fun: Box<dyn Fn(&U) -> T>,
  value: T,
}
