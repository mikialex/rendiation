pub mod window_state;
pub use window_state::*;

pub enum Value<T, U> {
  Static(T),
  Dynamic(DynamicValue<T, U>),
}

impl<T> Into<Value<String, T>> for &str {
  fn into(self) -> Value<String, T> {
    Value::Static(self.to_owned())
  }
}

impl<T, U> From<T> for Value<T, U> {
  fn from(v: T) -> Self {
    Self::Static(v)
  }
}

impl<T: Default, U> Value<T, U> {
  pub fn by(fun: impl Fn(&U) -> T + 'static) -> Self {
    Self::Dynamic(DynamicValue {
      fun: Box::new(fun),
      value: Default::default(),
    })
  }
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
