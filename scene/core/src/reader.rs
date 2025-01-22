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

    let pbr_mr = m.read_expected_foreign_key::<StandardModelRefPbrMRMaterial>(id);

    StandardModelDataView {
      material: SceneMaterialDataView::PbrMRMaterial(pbr_mr),
      mesh: m.read_expected_foreign_key::<StandardModelRefAttributesMeshEntity>(id),
    }
  }

  pub fn read_attribute_mesh(&self, id: EntityHandle<AttributesMeshEntity>) -> AttributesMesh {
    self.mesh.read(id).unwrap()
  }

  pub fn read_sampler(&self, id: EntityHandle<SceneSamplerEntity>) -> TextureSampler {
    self.sampler.read::<SceneSamplerInfo>(id)
  }
  pub fn read_texture(&self, id: EntityHandle<SceneTexture2dEntity>) -> GPUBufferImage {
    self
      .texture
      .read::<SceneTexture2dEntityDirectContent>(id)
      .unwrap()
      .ptr
      .as_ref()
      .clone()
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
      base_color_texture: Texture2DWithSamplingDataView::read::<PbrMRMaterialBaseColorTex, _>(
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
      albedo_texture: Texture2DWithSamplingDataView::read::<PbrSGMaterialAlbedoTex, _>(m, id),
      specular_texture: Texture2DWithSamplingDataView::read::<PbrSGMaterialSpecularTex, _>(m, id),
      glossiness_texture: Texture2DWithSamplingDataView::read::<PbrSGMaterialGlossinessTex, _>(
        m, id,
      ),
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
