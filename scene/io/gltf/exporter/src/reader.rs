use rendiation_texture_core::GPUBufferImage;
use virtual_collection::VirtualMultiCollection;

use crate::*;

pub struct SceneReader {
  mesh: AttributesMeshReader,
}

impl Default for SceneReader {
  fn default() -> Self {
    Self { mesh: todo!() }
  }
}

impl SceneReader {
  pub fn models(&self) -> impl Iterator<Item = EntityHandle<SceneModelEntity>> {
    [].into_iter()
  }

  pub fn read_node(&self, id: EntityHandle<SceneNodeEntity>) -> SceneNodeDataView {
    todo!()
  }

  pub fn read_scene_model(&self, id: EntityHandle<SceneModelEntity>) -> SceneModelDataView {
    todo!()
  }

  pub fn read_std_model(&self, id: EntityHandle<StandardModelEntity>) -> StandardModelDataView {
    todo!()
  }

  pub fn read_attribute_mesh(&self, id: EntityHandle<AttributesMeshEntity>) -> AttributesMesh {
    self.mesh.read(id).unwrap()
  }

  pub fn read_sampler(&self, id: EntityHandle<SceneSamplerEntity>) -> TextureSampler {
    todo!()
  }
  pub fn read_texture(&self, id: EntityHandle<SceneTexture2dEntity>) -> GPUBufferImage {
    todo!()
  }

  pub fn read_pbr_mr_material(
    &self,
    id: EntityHandle<PbrMRMaterialEntity>,
  ) -> PhysicalMetallicRoughnessMaterialDataView {
    todo!()
  }

  pub fn traverse_children_tree(
    &self,
    root: EntityHandle<SceneNodeEntity>,
    f: impl FnMut(EntityHandle<SceneNodeEntity>, Option<EntityHandle<SceneNodeEntity>>),
  ) {
    todo!()
  }
}

pub struct AttributesMeshReader {
  topology: ComponentReadView<AttributesMeshEntityTopology>,
  buffer: ComponentReadView<BufferEntityData>,
  mesh_ref_vertex: Box<
    dyn virtual_collection::DynVirtualMultiCollection<
      EntityHandle<AttributesMeshEntity>,
      EntityHandle<AttributesMeshEntityVertexBufferRelation>,
    >,
  >,
  semantic: ComponentReadView<AttributesMeshEntityVertexBufferSemantic>,

  index: SceneBufferViewReadView<AttributeIndexRef>,
  vertex: SceneBufferViewReadView<AttributeVertexRef>,
}

pub struct SceneBufferViewReadView<T: SceneBufferView> {
  pub range: ComponentReadView<SceneBufferViewBufferRange<T>>,
  pub count: ComponentReadView<SceneBufferViewBufferItemCount<T>>,
  pub buffer: ForeignKeyReadView<SceneBufferViewBufferId<T>>,
}

impl<T: SceneBufferView> SceneBufferViewReadView<T> {
  pub fn read_view(&self, id: EntityHandle<T::Entity>) -> Option<SceneBufferViewDataView> {
    SceneBufferViewDataView {
      data: self.buffer.get(id),
      range: self.range.get_value(id)?,
      count: self.count.get_value(id)?,
    }
    .into()
  }
}

pub fn scene_buffer_view_into_attribute(
  view: SceneBufferViewDataView,
  buffer: &ComponentReadView<BufferEntityData>,
) -> Option<AttributeAccessor> {
  let buffer = buffer.get_value(view.data?)?.ptr.clone();
  let range = view.range.unwrap_or_default();
  let count = view.count.unwrap() as usize;

  let byte_size = range
    .size
    .map(|size| u64::from(size) as usize)
    .unwrap_or(buffer.len());

  AttributeAccessor {
    view: UnTypedBufferView { buffer, range },
    byte_offset: range.offset as usize,
    count,
    item_byte_size: byte_size / count,
  }
  .into()
}

impl AttributesMeshReader {
  pub fn read(&self, id: EntityHandle<AttributesMeshEntity>) -> Option<AttributesMesh> {
    let mode = self.topology.get_value(id)?;

    let attributes = self
      .mesh_ref_vertex
      .access_multi(&id)?
      .map(|id| {
        let semantic = self.semantic.get_value(id).unwrap();
        let vertex = self.vertex.read_view(id).unwrap();
        let vertex = scene_buffer_view_into_attribute(vertex, &self.buffer).unwrap();
        (semantic, vertex)
      })
      .collect();

    let indices = self.index.read_view(id)?;
    let indices = scene_buffer_view_into_attribute(indices, &self.buffer).and_then(|i| {
      let fmt = match i.item_byte_size {
        4 => AttributeIndexFormat::Uint32,
        2 => AttributeIndexFormat::Uint16,
        _ => return None,
      };
      (fmt, i).into()
    });

    AttributesMesh {
      attributes,
      indices,
      mode,
      groups: MeshGroupsInfo::default(),
    }
    .into()
  }
}
