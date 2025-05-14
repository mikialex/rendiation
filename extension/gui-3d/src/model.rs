use crate::*;

pub struct UIWidgetModel {
  std_model: EntityHandle<StandardModelEntity>,
  model: EntityHandle<SceneModelEntity>,
  pub(crate) node: EntityHandle<SceneNodeEntity>,
  material: EntityHandle<UnlitMaterialEntity>,
  mesh: AttributesMeshEntities,
}

pub struct UiWidgetModelResponse {
  pub mouse_entering: bool,
  pub mouse_leave: bool,
  pub mouse_hovering: Option<HitPoint3D>,
  pub mouse_down: Option<HitPoint3D>,
  pub mouse_click: Option<HitPoint3D>,
}

impl UIWidgetModel {
  pub fn new(v: &mut SceneWriter, shape: AttributesMeshData) -> Self {
    let material = v.unlit_mat_writer.new_entity();
    let mesh = v.write_attribute_mesh(shape.build());
    let model = StandardModelDataView {
      material: SceneMaterialDataView::UnlitMaterial(material),
      mesh: mesh.mesh,
      skin: None,
    }
    .write(&mut v.std_model_writer);
    let node = v.node_writer.new_entity();
    let scene_model = SceneModelDataView {
      model,
      scene: v.scene,
      node,
    }
    .write(&mut v.model_writer);

    Self {
      std_model: model,
      model: scene_model,
      node,
      material,
      mesh,
    }
  }

  pub fn do_cleanup(&mut self, scene_cx: &mut SceneWriter) {
    scene_cx.std_model_writer.delete_entity(self.std_model);
    scene_cx.model_writer.delete_entity(self.model);
    scene_cx.node_writer.delete_entity(self.node);
    scene_cx.unlit_mat_writer.delete_entity(self.material);

    self
      .mesh
      .clean_up(&mut scene_cx.mesh_writer, &mut scene_cx.buffer_writer);
  }

  pub fn set_color(&mut self, cx3d: &mut SceneWriter, color: Vec3<f32>) -> &mut Self {
    cx3d
      .unlit_mat_writer
      .write::<UnlitMaterialColorComponent>(self.material, color.expand_with_one());
    self
  }
  pub fn set_visible(&mut self, cx3d: &mut SceneWriter, v: bool) -> &mut Self {
    cx3d
      .node_writer
      .write::<SceneNodeVisibleComponent>(self.node, v);
    self
  }

  pub fn set_matrix(&mut self, cx3d: &mut SceneWriter, mat: Mat4<f32>) -> &mut Self {
    cx3d
      .node_writer
      .write::<SceneNodeLocalMatrixComponent>(self.node, mat);
    self
  }
  /// find a macro to do this!
  pub fn with_matrix(mut self, cx3d: &mut SceneWriter, mat: Mat4<f32>) -> Self {
    self.set_matrix(cx3d, mat);
    self
  }

  /// return previous mesh entity for user decide if they want to delete it
  pub fn replace_shape(
    &mut self,
    cx3d: &mut SceneWriter,
    shape: AttributesMeshData,
  ) -> AttributesMeshEntities {
    let new_mesh = cx3d.write_attribute_mesh(shape.build());
    cx3d
      .std_model_writer
      .write_foreign_key::<StandardModelRefAttributesMeshEntity>(
        self.std_model,
        Some(new_mesh.mesh),
      );
    std::mem::replace(&mut self.mesh, new_mesh)
  }

  pub fn replace_new_shape_and_cleanup_old(
    &mut self,
    scene_cx: &mut SceneWriter,
    shape: AttributesMeshData,
  ) -> &mut Self {
    let old_mesh = self.replace_shape(scene_cx, shape);
    old_mesh.clean_up(&mut scene_cx.mesh_writer, &mut scene_cx.buffer_writer);
    self
  }

  pub fn with_parent(self, cx3d: &mut SceneWriter, parent: EntityHandle<SceneNodeEntity>) -> Self {
    cx3d
      .node_writer
      .write::<SceneNodeParentIdx>(self.node, Some(parent.into_raw()));
    self
  }
}
