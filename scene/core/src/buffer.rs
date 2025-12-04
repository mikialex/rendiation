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
    writer.new_entity(|w| w.write::<BufferEntityData>(&ExternalRefPtr::new(self)))
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
    // when improve this, make sure the attribute entities drop implementation should not be called
    // in some cases
    writer.new_entity(|w| {
      w.write::<BufferEntityData>(&ExternalRefPtr::new_shared(std::sync::Arc::new(
        self.view.buffer.get(start..end).unwrap().to_vec(),
      )))
    })
  }
}

pub trait SceneBufferView: EntityAssociateSemantic {}

#[derive(Clone)]
pub struct SceneBufferViewDataView {
  pub data: Option<EntityHandle<BufferEntity>>,
  pub range: Option<BufferViewRange>,
  pub count: u32,
}

impl SceneBufferViewDataView {
  pub fn write<C>(self, writer: EntityInitWriteView) -> EntityInitWriteView
  where
    C: SceneBufferView,
  {
    writer
      .write::<SceneBufferViewBufferId<C>>(&self.data.and_then(|v| v.some_handle()))
      .write::<SceneBufferViewBufferRange<C>>(&self.range)
      .write::<SceneBufferViewBufferItemCount<C>>(&self.count)
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
  type Data = u32;
}
