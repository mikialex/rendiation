use rendiation_texture_core::GPUBufferImage;

use crate::*;

pub struct SceneReader {
  pub scene_id: EntityHandle<SceneEntity>,
  pub scene_ref_models: RevRefOfForeignKey<SceneModelBelongsToScene>,

  pub mesh: AttributesMeshReader,

  pub node_reader: EntityReader<SceneNodeEntity>,
  pub node_children:
    BoxedDynMultiQuery<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
  pub camera: EntityReader<SceneCameraEntity>,
  pub scene_model: EntityReader<SceneModelEntity>,
  pub std_model: EntityReader<StandardModelEntity>,
  pub sampler: EntityReader<SceneSamplerEntity>,
  pub texture: EntityReader<SceneTexture2dEntity>,

  pub pbr_mr: EntityReader<PbrMRMaterialEntity>,
  pub pbr_sg: EntityReader<PbrSGMaterialEntity>,
  pub unlit: EntityReader<UnlitMaterialEntity>,
}

impl SceneReader {
  pub fn new_from_global(
    scene_id: EntityHandle<SceneEntity>,
    mesh_ref_vertex: RevRefOfForeignKey<
      AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity,
    >,
    node_children: BoxedDynMultiQuery<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
    scene_ref_models: RevRefOfForeignKey<SceneModelBelongsToScene>,
  ) -> Self {
    Self {
      scene_id,
      scene_ref_models,
      mesh: AttributesMeshReader::new_from_global(mesh_ref_vertex),
      node_reader: global_entity_of().entity_reader(),
      node_children,
      scene_model: global_entity_of().entity_reader(),
      camera: global_entity_of().entity_reader(),
      std_model: global_entity_of().entity_reader(),
      sampler: global_entity_of().entity_reader(),
      texture: global_entity_of().entity_reader(),
      pbr_mr: global_entity_of().entity_reader(),
      pbr_sg: global_entity_of().entity_reader(),
      unlit: global_entity_of().entity_reader(),
    }
  }

  pub fn models(&self) -> impl Iterator<Item = EntityHandle<SceneModelEntity>> + '_ {
    self.scene_ref_models.access_multi(&self.scene_id).unwrap()
  }

  pub fn read_node(&self, id: EntityHandle<SceneNodeEntity>) -> SceneNodeDataView {
    let node = &self.node_reader;
    SceneNodeDataView {
      visible: node.read::<SceneNodeVisibleComponent>(id),
      local_matrix: node.read::<SceneNodeLocalMatrixComponent>(id),
      parent: node.read::<SceneNodeParentIdx>(id),
    }
  }

  /// if only requires parent, this is cheaper compare to [Self::read_node]
  pub fn read_node_parent(
    &self,
    id: EntityHandle<SceneNodeEntity>,
  ) -> Option<EntityHandle<SceneNodeEntity>> {
    self.node_reader.read_foreign_key::<SceneNodeParentIdx>(id)
  }

  pub fn read_scene_model(&self, id: EntityHandle<SceneModelEntity>) -> SceneModelDataView {
    let sm = &self.scene_model;
    SceneModelDataView {
      model: sm.read_expected_foreign_key::<SceneModelStdModelRenderPayload>(id),
      scene: sm.read_expected_foreign_key::<SceneModelBelongsToScene>(id),
      node: sm.read_expected_foreign_key::<SceneModelRefNode>(id),
    }
  }

  pub fn read_std_model(&self, id: EntityHandle<StandardModelEntity>) -> StandardModelDataView {
    let m = &self.std_model;

    let material = m
      .read_foreign_key::<StandardModelRefPbrMRMaterial>(id)
      .map(SceneMaterialDataView::PbrMRMaterial)
      .or_else(|| {
        m.read_foreign_key::<StandardModelRefPbrSGMaterial>(id)
          .map(SceneMaterialDataView::PbrSGMaterial)
      })
      .or_else(|| {
        m.read_foreign_key::<StandardModelRefUnlitMaterial>(id)
          .map(SceneMaterialDataView::UnlitMaterial)
      })
      .unwrap();

    StandardModelDataView {
      material,
      mesh: m.read_expected_foreign_key::<StandardModelRefAttributesMeshEntity>(id),
      skin: m.read_foreign_key::<StandardModelRefSkin>(id),
    }
  }

  pub fn read_attribute_mesh(&self, id: EntityHandle<AttributesMeshEntity>) -> AttributesMesh {
    self
      .mesh
      .read(id)
      .unwrap()
      .try_into_attributes_mesh()
      .unwrap()
  }

  pub fn read_sampler(&self, id: EntityHandle<SceneSamplerEntity>) -> TextureSampler {
    self.sampler.read::<SceneSamplerInfo>(id)
  }

  pub fn read_texture(
    &self,
    id: EntityHandle<SceneTexture2dEntity>,
  ) -> Option<&MaybeUriData<Arc<GPUBufferImage>>> {
    let tex = self
      .texture
      .try_read_ref::<SceneTexture2dEntityDirectContent>(id)?
      .as_ref()?;
    tex.ptr.as_ref().into()
  }

  pub fn read_unlit_material(
    &self,
    id: EntityHandle<UnlitMaterialEntity>,
  ) -> UnlitMaterialDataView {
    let m = &self.unlit;
    UnlitMaterialDataView {
      color: m.read::<UnlitMaterialColorComponent>(id),
      color_alpha_tex: Texture2DWithSamplingDataView::read::<UnlitMaterialColorAlphaTex, _>(m, id),
      alpha: AlphaConfigDataView::read::<UnlitMaterialAlphaConfig, _>(m, id),
    }
  }

  pub fn read_pbr_mr_material(
    &self,
    id: EntityHandle<PbrMRMaterialEntity>,
  ) -> PhysicalMetallicRoughnessMaterialDataView {
    let m = &self.pbr_mr;
    PhysicalMetallicRoughnessMaterialDataView {
      base_color: m.read::<PbrMRMaterialBaseColorComponent>(id),
      roughness: m.read::<PbrMRMaterialRoughnessComponent>(id),
      metallic: m.read::<PbrMRMaterialMetallicComponent>(id),
      emissive: m.read::<PbrMRMaterialEmissiveComponent>(id),
      alpha: AlphaConfigDataView::read::<PbrMRMaterialAlphaConfig, _>(m, id),
      base_color_texture: Texture2DWithSamplingDataView::read::<PbrMRMaterialBaseColorAlphaTex, _>(
        m, id,
      ),
      metallic_roughness_texture: Texture2DWithSamplingDataView::read::<
        PbrMRMaterialMetallicRoughnessTex,
        _,
      >(m, id),
      emissive_texture: Texture2DWithSamplingDataView::read::<PbrMRMaterialEmissiveTex, _>(m, id),
      normal_texture: NormalMappingDataView::read::<PbrMRMaterialNormalInfo, _>(m, id),
    }
  }

  pub fn read_pbr_sg_material(
    &self,
    id: EntityHandle<PbrSGMaterialEntity>,
  ) -> PhysicalSpecularGlossinessMaterialDataView {
    let m = &self.pbr_sg;
    PhysicalSpecularGlossinessMaterialDataView {
      albedo: m.read::<PbrSGMaterialAlbedoComponent>(id),
      specular: m.read::<PbrSGMaterialSpecularComponent>(id),
      glossiness: m.read::<PbrSGMaterialGlossinessComponent>(id),
      emissive: m.read::<PbrSGMaterialEmissiveComponent>(id),
      alpha: AlphaConfigDataView::read::<PbrSGMaterialAlphaConfig, _>(m, id),
      emissive_texture: Texture2DWithSamplingDataView::read::<PbrSGMaterialEmissiveTex, _>(m, id),
      normal_texture: NormalMappingDataView::read::<PbrSGMaterialNormalInfo, _>(m, id),
      albedo_texture: Texture2DWithSamplingDataView::read::<PbrSGMaterialAlbedoAlphaTex, _>(m, id),
      specular_glossiness_texture: Texture2DWithSamplingDataView::read::<
        PbrSGMaterialSpecularGlossinessTex,
        _,
      >(m, id),
    }
  }

  pub fn traverse_children_tree(
    &self,
    root: EntityHandle<SceneNodeEntity>,
    f: &mut impl FnMut(EntityHandle<SceneNodeEntity>, Option<EntityHandle<SceneNodeEntity>>),
  ) {
    f(root, None);
    self.traverse_children_tree_impl(root, f)
  }

  fn traverse_children_tree_impl(
    &self,
    root: EntityHandle<SceneNodeEntity>,
    f: &mut impl FnMut(EntityHandle<SceneNodeEntity>, Option<EntityHandle<SceneNodeEntity>>),
  ) {
    if let Some(children) = self.node_children.access_multi(&root) {
      for child in children {
        f(child, Some(root));
        self.traverse_children_tree_impl(child, f);
      }
    }
  }
}

pub struct SceneBufferViewReadView<T: SceneBufferView> {
  pub range: ComponentReadView<SceneBufferViewBufferRange<T>>,
  pub count: ComponentReadView<SceneBufferViewBufferItemCount<T>>,
  pub buffer: ForeignKeyReadView<SceneBufferViewBufferId<T>>,
}

impl<T: SceneBufferView> SceneBufferViewReadView<T> {
  pub fn new_from_global() -> Self {
    Self {
      range: global_database().read(),
      count: global_database().read(),
      buffer: global_database().read_foreign_key(),
    }
  }

  pub fn read_view(&self, id: EntityHandle<T::Entity>) -> Option<SceneBufferViewDataView> {
    SceneBufferViewDataView {
      data: self.buffer.get(id),
      range: self.range.get_value(id)?,
      count: self.count.get_value(id)?,
    }
    .into()
  }

  pub fn read_maybe_uri(
    &self,
    id: EntityHandle<T::Entity>,
    buffer_reader: &ComponentReadView<BufferEntityData>,
  ) -> Option<AttributeUriData> {
    let range = self.range.get_value(id)?;
    let count = self.count.get_value(id)? as usize;
    let data = self.buffer.get(id)?;
    let data = buffer_reader.get(data)?.ptr.as_ref().clone();
    AttributeUriData { data, range, count }.into()
  }

  /// return (data, count)
  pub fn read_view_bytes<'a>(
    &self,
    id: EntityHandle<T::Entity>,
    buffer_reader: &'a ComponentReadView<BufferEntityData>,
  ) -> Option<(&'a [u8], u32)> {
    let range = self.range.get_value(id)?;
    let count = self.count.get_value(id)?;
    let data = self.buffer.get(id)?;

    let data = buffer_reader.get(data)?;
    let data = data.as_living()?.as_slice();

    if let Some(range) = range {
      let range = range.into_range(data.len());
      let data = data.get(range)?;
      (data, count)
    } else {
      (data, count)
    }
    .into()
  }

  pub fn read_view_slice<'a, V: Pod>(
    &self,
    id: EntityHandle<T::Entity>,
    buffer_reader: &'a ComponentReadView<BufferEntityData>,
  ) -> Option<&'a [V]> {
    let (bytes, count) = self.read_view_bytes(id, buffer_reader)?;
    let byte_per_item = bytes.len() / count as usize;
    assert_eq!(byte_per_item, std::mem::size_of::<V>());
    bytemuck::try_cast_slice(bytes).ok()
  }
}

pub fn scene_buffer_view_into_attribute(
  view: SceneBufferViewDataView,
  buffer: &ComponentReadView<BufferEntityData>,
) -> Option<AttributeAccessor> {
  let data = buffer.get(view.data?)?;
  let buffer = data.ptr.as_ref().clone().into_living()?;

  let range = view.range.unwrap_or_default();
  let count = view.count as usize;

  let byte_size = range
    .size
    .map(|size| u64::from(size) as usize)
    .unwrap_or(buffer.len());

  AttributeAccessor {
    view: UnTypedBufferView { buffer, range },
    byte_offset: 0,
    count,
    item_byte_size: byte_size / count,
  }
  .into()
}

pub struct AttributesMeshReader {
  topology: ComponentReadView<AttributesMeshEntityTopology>,
  buffer: ComponentReadView<BufferEntityData>,
  semantic: ComponentReadView<AttributesMeshEntityVertexBufferSemantic>,
  mesh_ref_vertex:
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,

  index: SceneBufferViewReadView<AttributeIndexRef>,
  vertex: SceneBufferViewReadView<AttributeVertexRef>,
}

impl AttributesMeshReader {
  pub fn new_from_global(
    mesh_ref_vertex: RevRefOfForeignKey<
      AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity,
    >,
  ) -> Self {
    Self {
      mesh_ref_vertex,
      topology: global_database().read(),
      buffer: global_database().read(),
      semantic: global_database().read(),
      index: SceneBufferViewReadView::new_from_global(),
      vertex: SceneBufferViewReadView::new_from_global(),
    }
  }

  pub fn read(&self, id: EntityHandle<AttributesMeshEntity>) -> Option<AttributesMeshWithUri> {
    let mode = self.topology.get_value(id)?;

    let vertices = self
      .mesh_ref_vertex
      .access_multi(&id)?
      .map(|id| {
        let semantic = self.semantic.get_value(id).unwrap();
        let data = self.vertex.read_maybe_uri(id, &self.buffer).unwrap();

        AttributeMeshUriVertex {
          relation_handle: id.into_raw(),
          semantic,
          data,
        }
      })
      .collect();

    let indices = self.index.read_maybe_uri(id, &self.buffer);

    AttributesMeshWithUri {
      vertices,
      indices,
      mode,
    }
    .into()
  }
}

#[derive(Clone)] // todo, consider remove clone
pub struct AttributesMeshWithUri {
  pub mode: PrimitiveTopology,
  pub indices: Option<AttributeUriData>,
  pub vertices: Vec<AttributeMeshUriVertex>,
}

#[derive(Clone)]
pub struct AttributeMeshUriVertex {
  pub relation_handle: RawEntityHandle,
  pub semantic: AttributeSemantic,
  pub data: AttributeUriData,
}

#[derive(Clone)]
pub struct AttributeUriData {
  pub data: BufferEntityDataType,
  pub range: Option<BufferViewRange>,
  pub count: usize,
}

impl AttributeUriData {
  pub fn expect_living(self) -> AttributeLivingData {
    AttributeLivingData {
      data: self.data.into_living().unwrap(),
      range: self.range,
      count: self.count,
    }
  }
}

#[derive(Clone)] // todo, consider remove clone
pub struct AttributesMeshWithVertexRelationInfo {
  pub mode: PrimitiveTopology,
  pub indices: Option<AttributeLivingData>,
  pub vertices: Vec<AttributeMeshLivingVertex>,
}

impl AttributesMeshWithVertexRelationInfo {
  pub fn into_attributes_mesh(self) -> AttributesMesh {
    let AttributesMeshWithVertexRelationInfo {
      mode,
      indices,
      vertices,
    } = self;

    let vertices = vertices
      .into_iter()
      .map(|v| {
        let view = v.data.into_accessor();
        (v.semantic, view)
      })
      .collect();

    let indices = indices.map(|i| {
      let view = i.into_accessor();
      let fmt = match view.item_byte_size {
        4 => AttributeIndexFormat::Uint32,
        2 => AttributeIndexFormat::Uint16,
        _ => unreachable!("invalid index fmt"),
      };
      (fmt, view)
    });

    AttributesMesh {
      attributes: vertices,
      indices,
      mode,
    }
  }
}

#[derive(Clone)]
pub struct AttributeMeshLivingVertex {
  pub relation_handle: RawEntityHandle,
  pub semantic: AttributeSemantic,
  pub data: AttributeLivingData,
}

#[derive(Clone)]
pub struct AttributeLivingData {
  pub data: Arc<Vec<u8>>,
  pub range: Option<BufferViewRange>,
  pub count: usize,
}

impl AttributeLivingData {
  pub fn into_accessor(self) -> AttributeAccessor {
    let range = self.range.unwrap_or_default();
    let count = self.count;

    let byte_size = range
      .size
      .map(|size| u64::from(size) as usize)
      .unwrap_or(self.data.len());

    AttributeAccessor {
      view: UnTypedBufferView {
        buffer: self.data,
        range,
      },
      byte_offset: 0,
      count,
      item_byte_size: byte_size / count,
    }
  }
}

impl AttributesMeshWithUri {
  pub fn try_into_attributes_mesh(self) -> Option<AttributesMesh> {
    match self.into_maybe_uri_form() {
      MaybeUriData::Uri(_) => None,
      MaybeUriData::Living(mesh) => Some(mesh.into_attributes_mesh()),
    }
  }

  pub fn into_maybe_uri_form(
    self,
  ) -> MaybeUriData<AttributesMeshWithVertexRelationInfo, AttributesMeshWithUri> {
    for d in &self.vertices {
      if d.data.data.as_living().is_none() {
        return MaybeUriData::Uri(self);
      }
    }

    if let Some(indices) = &self.indices {
      if indices.data.as_living().is_none() {
        return MaybeUriData::Uri(self);
      }
    }

    let attributes = self
      .vertices
      .into_iter()
      .map(|v| {
        let data = v.data.expect_living();
        AttributeMeshLivingVertex {
          relation_handle: v.relation_handle,
          semantic: v.semantic,
          data,
        }
      })
      .collect();

    let indices = self.indices.map(|v| v.expect_living());

    MaybeUriData::Living(AttributesMeshWithVertexRelationInfo {
      mode: self.mode,
      indices,
      vertices: attributes,
    })
  }
}
