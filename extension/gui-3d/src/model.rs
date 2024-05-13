use crate::*;

pub struct UIWidgetModel {
  /// indicate if this widget is interactive to mouse event
  mouse_interactive: bool,

  is_mouse_in: bool,
  is_mouse_in_and_down: bool,

  on_mouse_click: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,
  on_mouse_hovering: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,
  on_mouse_in: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,
  on_mouse_out: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,
  on_mouse_down: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,

  parent: Option<AllocIdx<SceneNodeEntity>>,
  model: AllocIdx<SceneModelEntity>,
  node: AllocIdx<SceneNodeEntity>,
  material: AllocIdx<FlatMaterialEntity>,
  mesh: AllocIdx<AttributeMeshEntity>,
}

impl View for UIWidgetModel {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    // if let Some(update) = self.view_update {
    //   // update(self, model)
    // }
  }
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    if self.mouse_interactive && self.has_any_mouse_event() {
      if let Some(hit) = cx.picker.pick_model_nearest(self.model, cx.mouse_world_ray) {
        // emit
      }
    }
    //
  }
  fn clean_up(&mut self, cx: &mut StateStore) {
    todo!();
  }
}

impl UIWidgetModel {
  pub fn new(v: &mut View3dProvider) -> Self {
    todo!()
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

  pub fn with_on_mouse_click(
    mut self,
    f: impl FnMut(&mut View3dStateUpdateCtx, Vec3<f32>) + 'static,
  ) -> Self {
    self.on_mouse_click = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_in(
    mut self,
    f: impl FnMut(&mut View3dStateUpdateCtx, Vec3<f32>) + 'static,
  ) -> Self {
    self.on_mouse_in = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_out(
    mut self,
    f: impl FnMut(&mut View3dStateUpdateCtx, Vec3<f32>) + 'static,
  ) -> Self {
    self.on_mouse_out = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_hovering(
    mut self,
    f: impl FnMut(&mut View3dStateUpdateCtx, Vec3<f32>) + 'static,
  ) -> Self {
    self.on_mouse_hovering = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_down(
    mut self,
    f: impl FnMut(&mut View3dStateUpdateCtx, Vec3<f32>) + 'static,
  ) -> Self {
    self.on_mouse_down = Some(Box::new(f));
    self
  }

  pub fn set_color(&mut self, cx3d: &mut View3dProvider, color: Vec3<f32>) -> &mut Self {
    global_entity_component_of::<FlatMaterialDisplayColorComponent>()
      .write()
      .write(self.material.index, color.expand_with_one());
    self
  }
  pub fn set_visible(&mut self, cx3d: &mut View3dProvider, v: bool) -> &mut Self {
    self
  }

  pub fn set_matrix(&mut self, cx3d: &mut View3dProvider, mat: Mat4<f32>) -> &mut Self {
    self
  }
  /// find a macro to do this!
  pub fn with_matrix(mut self, cx3d: &mut View3dProvider, mat: Mat4<f32>) -> Self {
    self.set_matrix(cx3d, mat);
    self
  }

  pub fn with_shape(mut self, cx3d: &mut View3dProvider, shape: AttributesMeshData) -> Self {
    self
  }

  pub fn with_parent(
    mut self,
    cx3d: &mut View3dProvider,
    parent: AllocIdx<SceneNodeEntity>,
  ) -> Self {
    self
  }
}
