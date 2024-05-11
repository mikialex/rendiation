use crate::*;

pub struct UIModel {
  /// indicate if this widget is interactive to mouse event
  mouse_interactive: bool,

  is_mouse_in: bool,
  is_mouse_in_and_down: bool,

  on_mouse_click: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,
  on_mouse_hovering: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,
  on_mouse_down: Option<Box<dyn FnMut(&mut View3dStateUpdateCtx, Vec3<f32>)>>,

  model: AllocIdx<SceneModelEntity>,
  nodes: AllocIdx<SceneNodeEntity>,
  material: AllocIdx<FlatMaterialEntity>,
  mesh: AllocIdx<AttributeMeshEntity>,

  view_update: Option<Box<dyn FnMut(&mut Self, &mut StateStore)>>,
}

impl Default for UIModel {
  fn default() -> Self {
    todo!()
  }
}

impl View for UIModel {
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
}

impl UIModel {
  fn has_any_mouse_event(&self) -> bool {
    self.on_mouse_click.is_some()
      || self.on_mouse_hovering.is_some()
      || self.on_mouse_down.is_some()
  }

  pub fn with_view_update(mut self, f: impl FnMut(&mut Self, &mut StateStore) + 'static) -> Self {
    self.view_update = Some(Box::new(f));
    self
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

  pub fn set_color(&mut self, color: Vec3<f32>) -> &mut Self {
    self
  }
  pub fn set_visible(&mut self, v: bool) -> &mut Self {
    self
  }

  pub fn set_matrix(&mut self, mat: Mat4<f32>) -> &mut Self {
    self
  }
  /// find a macro to do this!
  pub fn with_matrix(mut self, mat: Mat4<f32>) -> Self {
    self
  }

  pub fn with_shape(mut self, shape: AttributesMeshData) -> Self {
    self
  }
}
