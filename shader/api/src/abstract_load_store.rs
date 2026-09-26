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

impl ShaderAbstractLeftValue for () {
  type RightValue = ();
  fn abstract_load(&self) -> Self::RightValue {}
  fn abstract_store(&self, _: Self::RightValue) {}
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

pub struct ShaderAccessorAsAbstractLeftValue<T: ShaderSizedValueNodeType>(pub ShaderPtrOf<T>);

impl<T: ShaderSizedValueNodeType> ShaderAbstractLeftValue for ShaderAccessorAsAbstractLeftValue<T> {
  type RightValue = Node<T>;
  fn abstract_load(&self) -> Node<T> {
    self.0.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.0.store(payload)
  }
}

pub trait ShaderAbstractRightValue: Clone + 'static {
  type AbstractLeftValue: ShaderAbstractLeftValue<RightValue = Self>;
  /// the value stored in left value can not be assumed uninit or zeroed, and
  /// it should be related to the context of usage
  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue;
}

impl ShaderAbstractRightValue for () {
  type AbstractLeftValue = ();

  fn create_left_value_from_builder<B: LeftValueBuilder>(_: &mut B) -> Self::AbstractLeftValue {}
}

impl<T: ShaderSizedValueNodeType> ShaderAbstractRightValue for Node<T> {
  type AbstractLeftValue = BoxedShaderLoadStore<Node<T>>;
  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue {
    builder.create_single_left_value()
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
    // the initial value can not be assumed, so the explicit zero store is not required
    Box::new(ShaderAccessorAsAbstractLeftValue(make_local_var::<T>()))
  }
}

/// The left value of the expanded shader struct ([ENode]), stored as the struct node. The
/// `#[shader_struct]` implements the [ShaderAbstractRightValue] for the [ENode] by it.
pub struct ENodeLeftValue<T>(pub BoxedShaderLoadStore<Node<T>>);

impl<T> ShaderAbstractLeftValue for ENodeLeftValue<T>
where
  T: ShaderStructuralNodeType + ShaderSizedValueNodeType,
{
  type RightValue = ENode<T>;

  fn abstract_load(&self) -> Self::RightValue {
    T::expand(self.0.abstract_load())
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.0.abstract_store(T::construct(payload))
  }
}

macro_rules! impl_tuple_abstract_value {
  ($($name: ident $index: tt),+) => {
    impl<$($name: ShaderAbstractLeftValue),+> ShaderAbstractLeftValue for ($($name,)+) {
      type RightValue = ($($name::RightValue,)+);

      fn abstract_load(&self) -> Self::RightValue {
        ($(self.$index.abstract_load(),)+)
      }

      fn abstract_store(&self, payload: Self::RightValue) {
        $(self.$index.abstract_store(payload.$index);)+
      }
    }

    impl<$($name: ShaderAbstractRightValue),+> ShaderAbstractRightValue for ($($name,)+) {
      type AbstractLeftValue = ($($name::AbstractLeftValue,)+);

      fn create_left_value_from_builder<Builder: LeftValueBuilder>(
        builder: &mut Builder,
      ) -> Self::AbstractLeftValue {
        ($($name::create_left_value_from_builder(builder),)+)
      }
    }
  };
}

impl_tuple_abstract_value!(A 0, B 1);
impl_tuple_abstract_value!(A 0, B 1, C 2);
impl_tuple_abstract_value!(A 0, B 1, C 2, D 3);
impl_tuple_abstract_value!(A 0, B 1, C 2, D 3, E 4);
impl_tuple_abstract_value!(A 0, B 1, C 2, D 3, E 4, F 5);
impl_tuple_abstract_value!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
impl_tuple_abstract_value!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
