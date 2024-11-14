use rendiation_mesh_core::BufferViewRange;

use crate::*;

declare_entity!(BufferEntity);
declare_component!(BufferEntityData, BufferEntity, ExternalRefPtr<Vec<u8>>);

impl EntityCustomWrite<BufferEntity> for Vec<u8> {
  type Writer = EntityWriter<BufferEntity>;

  fn create_writer() -> Self::Writer {
    global_entity_of::<BufferEntity>().entity_writer()
  }

  fn write(self, writer: &mut Self::Writer) -> EntityHandle<BufferEntity> {
    writer
      .component_value_writer::<BufferEntityData>(ExternalRefPtr::new(self))
      .new_entity()
  }
}

impl EntityCustomWrite<BufferEntity> for AttributeAccessor {
  type Writer = EntityWriter<BufferEntity>;

  fn create_writer() -> Self::Writer {
    global_entity_of::<BufferEntity>().entity_writer()
  }

  fn write(self, writer: &mut Self::Writer) -> EntityHandle<BufferEntity> {
    let start = self.byte_offset + self.view.range.offset as usize;
    let end = start + self.count * self.item_byte_size;
    // for simplicity we clone out sub buffer, this should be improved
    writer
      .component_value_writer::<BufferEntityData>(ExternalRefPtr::new_shared(std::sync::Arc::new(
        self.view.buffer.get(start..end).unwrap().to_vec(),
      )))
      .new_entity()
  }
}

pub trait SceneBufferView: EntityAssociateSemantic {}

#[derive(Clone)]
pub struct SceneBufferViewDataView {
  pub data: Option<EntityHandle<BufferEntity>>,
  pub range: Option<BufferViewRange>,
  pub count: Option<u32>,
}

impl SceneBufferViewDataView {
  pub fn write<C, E>(self, writer: &mut EntityWriter<E>)
  where
    E: EntitySemantic,
    C: SceneBufferView,
    C: EntityAssociateSemantic<Entity = E>,
  {
    writer
      .component_value_writer::<SceneBufferViewBufferId<C>>(self.data.and_then(|v| v.some_handle()))
      .component_value_writer::<SceneBufferViewBufferRange<C>>(self.range)
      .component_value_writer::<SceneBufferViewBufferItemCount<C>>(self.count);
  }
}

pub fn register_scene_buffer_view<T: SceneBufferView>(
  ecg: EntityComponentGroupTyped<T::Entity>,
) -> EntityComponentGroupTyped<T::Entity> {
  ecg
    .declare_foreign_key::<SceneBufferViewBufferId<T>>()
    .declare_component::<SceneBufferViewBufferRange<T>>()
    .declare_component::<SceneBufferViewBufferItemCount<T>>()
}

pub struct SceneBufferViewBufferId<T>(T);
impl<T: SceneBufferView> EntityAssociateSemantic for SceneBufferViewBufferId<T> {
  type Entity = T::Entity;
}
impl<T: SceneBufferView> ComponentSemantic for SceneBufferViewBufferId<T> {
  type Data = ForeignKeyComponentData;
}
impl<T: SceneBufferView> ForeignKeySemantic for SceneBufferViewBufferId<T> {
  type ForeignEntity = BufferEntity;
}

/// if range is none, it means the whole buffer
pub struct SceneBufferViewBufferRange<T>(T);
impl<T: SceneBufferView> EntityAssociateSemantic for SceneBufferViewBufferRange<T> {
  type Entity = T::Entity;
}
impl<T: SceneBufferView> ComponentSemantic for SceneBufferViewBufferRange<T> {
  type Data = Option<BufferViewRange>;
}

pub struct SceneBufferViewBufferItemCount<T>(T);
impl<T: SceneBufferView> EntityAssociateSemantic for SceneBufferViewBufferItemCount<T> {
  type Entity = T::Entity;
}
impl<T: SceneBufferView> ComponentSemantic for SceneBufferViewBufferItemCount<T> {
  type Data = Option<u32>;
}
