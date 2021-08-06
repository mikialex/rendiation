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

pub struct ValueDiffResult<'a, T> {
  pub value: &'a T,
  pub changed: bool,
}

impl<T: Default, U> Value<T, U> {
  pub fn by(fun: impl Fn(&U) -> T + 'static) -> Self {
    Self::Dynamic(DynamicValue {
      fun: Box::new(fun),
      value: Default::default(),
    })
  }

  pub fn eval(&mut self, ctx: &U) -> &T {
    match self {
      Value::Static(v) => v,
      Value::Dynamic(d) => {
        d.value = (d.fun)(ctx);
        &d.value
      }
    }
  }

  pub fn diff_eval(&mut self, ctx: &U) -> ValueDiffResult<T>
  where
    T: PartialEq,
  {
    match self {
      Value::Static(value) => ValueDiffResult {
        value,
        changed: false,
      },
      Value::Dynamic(d) => {
        let new_value = (d.fun)(ctx);
        let changed = d.value != new_value;
        d.value = new_value;
        ValueDiffResult {
          value: &d.value,
          changed,
        }
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
