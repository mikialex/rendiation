use rendiation_mesh_core::BufferViewRange;

use crate::*;

declare_entity!(BufferEntity);
declare_component!(BufferEntityData, BufferEntity, ExternalRefPtr<Vec<u8>>);

pub trait SceneBufferView: EntityAssociateSemantic {}

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
  type Data = Option<u32>;
}
impl<T: SceneBufferView> ForeignKeySemantic for SceneBufferViewBufferId<T> {
  type ForeignEntity = BufferEntity;
}

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
