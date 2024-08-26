use crate::*;

/// abstract left value in shader
pub trait ShaderAbstractLeftValue {
  /// Value must a pure right value in shader (nested pointer is not allowed)
  ///
  /// todo, should we consider add right value bound for this?
  type RightValue;
  fn abstract_load(&self) -> Self::RightValue;
  fn abstract_store(&self, payload: Self::RightValue);
}

pub type BoxedShaderLoadStore<T> = Box<dyn ShaderAbstractLeftValue<RightValue = T>>;

impl<T> ShaderAbstractLeftValue for BoxedShaderLoadStore<T>
where
  T: ShaderAbstractRightValue<AbstractLeftValue = Self>,
{
  type RightValue = T;

  fn abstract_load(&self) -> Self::RightValue {
    (**self).abstract_load()
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    (**self).abstract_store(payload)
  }
}

impl<T: ShaderSizedValueNodeType> ShaderAbstractLeftValue for LocalVarNode<T> {
  type RightValue = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    self.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.store(payload)
  }
}

pub trait ShaderAbstractRightValue: Copy + 'static {
  type AbstractLeftValue: ShaderAbstractLeftValue<RightValue = Self>;
  /// the value stored in left value can not be assumed uninit or zeroed, and
  /// it should be related to the context of usage
  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue;
}

impl<T: ShaderSizedValueNodeType> ShaderAbstractRightValue for Node<T> {
  type AbstractLeftValue = BoxedShaderLoadStore<Node<T>>;
  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue {
    builder.create_single_left_value_zeroed()
  }
}

pub trait LeftValueBuilder: Sized {
  /// the value stored in left value can not be assumed uninit or zeroed, and
  /// it should be related to the context of usage
  fn create_single_left_value<T: ShaderSizedValueNodeType>(
    &mut self,
  ) -> BoxedShaderLoadStore<Node<T>>;

  fn create_single_left_value_zeroed<T: ShaderSizedValueNodeType>(
    &mut self,
  ) -> BoxedShaderLoadStore<Node<T>> {
    let v = self.create_single_left_value();
    v.abstract_store(zeroed_val());
    v
  }

  fn create_left_value<V: ShaderAbstractRightValue>(&mut self, right: V) -> V::AbstractLeftValue {
    let v = V::create_left_value_from_builder(self);
    v.abstract_store(right);
    v
  }
}

pub struct LocalLeftValueBuilder;

impl LeftValueBuilder for LocalLeftValueBuilder {
  fn create_single_left_value<T: ShaderSizedValueNodeType>(
    &mut self,
  ) -> BoxedShaderLoadStore<Node<T>> {
    Box::new(zeroed_val().make_local_var())
  }
}

// should impl for other tuple!
impl<A: ShaderAbstractLeftValue, B: ShaderAbstractLeftValue> ShaderAbstractLeftValue for (A, B) {
  type RightValue = (A::RightValue, B::RightValue);

  fn abstract_load(&self) -> Self::RightValue {
    (self.0.abstract_load(), self.1.abstract_load())
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.0.abstract_store(payload.0);
    self.1.abstract_store(payload.1);
  }
}

impl<A: ShaderAbstractRightValue, B: ShaderAbstractRightValue> ShaderAbstractRightValue for (A, B) {
  type AbstractLeftValue = (A::AbstractLeftValue, B::AbstractLeftValue);

  fn create_left_value_from_builder<Builder: LeftValueBuilder>(
    builder: &mut Builder,
  ) -> Self::AbstractLeftValue {
    (
      A::create_left_value_from_builder(builder),
      B::create_left_value_from_builder(builder),
    )
  }
}
