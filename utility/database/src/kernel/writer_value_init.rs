use crate::*;

// https://users.rust-lang.org/t/why-closure-cannot-return-a-reference-to-data-moved-into-closure/72655/6
pub trait ComponentInitValueProvider {
  type Value;
  fn next_value(&mut self) -> Option<&Self::Value>;
}

pub struct ValueAsInitValueProviderPersist<T>(pub T);

impl<T> ComponentInitValueProvider for ValueAsInitValueProviderPersist<T> {
  type Value = T;
  fn next_value(&mut self) -> Option<&Self::Value> {
    Some(&self.0)
  }
}
pub struct ValueAsInitValueProviderOnce<T>(T, bool);

impl<T> ValueAsInitValueProviderOnce<T> {
  pub fn new(value: T) -> Self {
    Self(value, false)
  }
}

impl<T> ComponentInitValueProvider for ValueAsInitValueProviderOnce<T> {
  type Value = T;
  fn next_value(&mut self) -> Option<&Self::Value> {
    if self.1 {
      None
    } else {
      self.1 = true;
      Some(&self.0)
    }
  }
}

/// just the helper trait for [ComponentInitValueProvider]
pub trait UntypedComponentInitValueProvider {
  fn next_value(&mut self) -> Option<DataPtr>;
}

pub(crate) struct UntypedComponentInitValueProviderImpl<T>(pub T);

impl<T: ComponentInitValueProvider> UntypedComponentInitValueProvider
  for UntypedComponentInitValueProviderImpl<T>
{
  fn next_value(&mut self) -> Option<DataPtr> {
    let next = self.0.next_value();
    next.map(|next| next as *const T::Value as DataPtr)
  }
}

#[test]
fn test_entity_writer_init_works() {
  setup_global_database(Default::default());

  declare_entity!(MyTestEntity);
  declare_component!(TestEntityFieldA, MyTestEntity, u32);
  declare_component!(TestEntityFieldB, MyTestEntity, f32);

  global_database()
    .declare_entity::<MyTestEntity>()
    .declare_component::<TestEntityFieldA>()
    .declare_component::<TestEntityFieldB>();

  {
    let ptr = global_entity_of::<MyTestEntity>()
      .entity_writer()
      .with_component_value_writer::<TestEntityFieldA>(2)
      .new_entity();

    let read_view = global_entity_component_of::<TestEntityFieldA>().read();
    assert_eq!(read_view.get(ptr), Some(&2_u32));
  }

  let mut writer = global_entity_of::<MyTestEntity>().entity_writer();
  writer.component_value_persist_writer::<TestEntityFieldA>(4);

  let ptr2 = writer.new_entity();
  let ptr3 = writer.new_entity();

  drop(writer);

  let read_view = global_entity_component_of::<TestEntityFieldA>().read();
  assert_eq!(read_view.get(ptr2), Some(&4_u32));
  assert_eq!(read_view.get(ptr3), Some(&4_u32));
}
