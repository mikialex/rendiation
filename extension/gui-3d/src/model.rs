use crate::*;

pub struct UIWidgetModel {
  /// indicate if this widget is interactive to mouse event
  mouse_interactive: bool,

  is_mouse_in: bool,
  is_mouse_in_and_down: bool,

  on_mouse_click: Option<Box<dyn FnMut(&mut StateCx, Vec3<f32>)>>,
  on_mouse_hovering: Option<Box<dyn FnMut(&mut StateCx, Vec3<f32>)>>,
  on_mouse_in: Option<Box<dyn FnMut(&mut StateCx, Vec3<f32>)>>,
  on_mouse_out: Option<Box<dyn FnMut(&mut StateCx, Vec3<f32>)>>,
  on_mouse_down: Option<Box<dyn FnMut(&mut StateCx, Vec3<f32>)>>,

  parent: Option<AllocIdx<SceneNodeEntity>>,
  std_model: AllocIdx<StandardModelEntity>,
  model: AllocIdx<SceneModelEntity>,
  node: AllocIdx<SceneNodeEntity>,
  material: AllocIdx<FlatMaterialEntity>,
  mesh: AllocIdx<AttributeMeshEntity>,
}

impl Widget for UIWidgetModel {
  fn update_view(&mut self, cx: &mut StateCx) {
    // if let Some(update) = self.view_update {
    //   // update(self, model)
    // }
  }
  fn update_state(&mut self, cx: &mut StateCx) {
    state_access!(cx, interaction_cx, InteractionState3d);
    if self.mouse_interactive && self.has_any_mouse_event() {
      if let Some(hit) = interaction_cx
        .picker
        .pick_model_nearest(self.model, interaction_cx.mouse_world_ray)
      {
        // emit
      }
    }
  }
  fn clean_up(&mut self, cx: &mut StateCx) {
    todo!();
  }
}

impl UIWidgetModel {
  pub fn new(v: &mut Scene3dWriter) -> Self {
    Self {
      mouse_interactive: true,
      is_mouse_in: false,
      is_mouse_in_and_down: false,
      on_mouse_click: None,
      on_mouse_hovering: None,
      on_mouse_in: None,
      on_mouse_out: None,
      on_mouse_down: None,
      parent: None,
      std_model: v.std_model_w.new_entity().alloc_idx(),
      model: v.model_writer.new_entity().alloc_idx(),
      node: v.node_writer.new_entity().alloc_idx(),
      material: v.flat_mat_writer.new_entity().alloc_idx(),
      mesh: todo!(),
    }
  }

  fn has_any_mouse_event(&self) -> bool {
    self.on_mouse_click.is_some()
      || self.on_mouse_hovering.is_some()
      || self.on_mouse_down.is_some()
  }

  pub fn set_mouse_interactive(&mut self, v: bool) -> &mut Self {
    self.mouse_interactive = v;
    if !self.mouse_interactive {
      self.is_mouse_in = false;
      self.is_mouse_in_and_down = false
    }
    self
  }

  pub fn with_on_mouse_click(mut self, f: impl FnMut(&mut StateCx, Vec3<f32>) + 'static) -> Self {
    self.on_mouse_click = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_in(mut self, f: impl FnMut(&mut StateCx, Vec3<f32>) + 'static) -> Self {
    self.on_mouse_in = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_out(mut self, f: impl FnMut(&mut StateCx, Vec3<f32>) + 'static) -> Self {
    self.on_mouse_out = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_hovering(
    mut self,
    f: impl FnMut(&mut StateCx, Vec3<f32>) + 'static,
  ) -> Self {
    self.on_mouse_hovering = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_down(mut self, f: impl FnMut(&mut StateCx, Vec3<f32>) + 'static) -> Self {
    self.on_mouse_down = Some(Box::new(f));
    self
  }

  pub fn set_color(&mut self, cx3d: &mut Scene3dWriter, color: Vec3<f32>) -> &mut Self {
    cx3d
      .flat_mat_writer
      .write_component_data::<FlatMaterialDisplayColorComponent>(
        self.material,
        color.expand_with_one(),
      );
    self
  }
  pub fn set_visible(&mut self, cx3d: &mut Scene3dWriter, v: bool) -> &mut Self {
    cx3d
      .node_writer
      .write_component_data::<SceneNodeVisibleComponent>(self.node, v);
    self
  }

  pub fn set_matrix(&mut self, cx3d: &mut Scene3dWriter, mat: Mat4<f32>) -> &mut Self {
    cx3d
      .node_writer
      .write_component_data::<SceneNodeLocalMatrixComponent>(self.node, mat);
    self
  }
  /// find a macro to do this!
  pub fn with_matrix(mut self, cx3d: &mut Scene3dWriter, mat: Mat4<f32>) -> Self {
    self.set_matrix(cx3d, mat);
    self
  }

  pub fn with_shape(mut self, cx3d: &mut Scene3dWriter, shape: AttributesMeshData) -> Self {
    self.mesh = cx3d.write_attribute_mesh(shape.build()).alloc_idx();
    self
  }

  pub fn with_parent(self, cx3d: &mut Scene3dWriter, parent: AllocIdx<SceneNodeEntity>) -> Self {
    cx3d
      .node_writer
      .write_component_data::<SceneNodeParentIdx>(self.node, Some(parent.index));
    self
  }
}
