use crate::*;

pub struct UIWidgetModel {
  /// indicate if this widget is interactive to mouse event
  mouse_interactive: bool,
  registered_interactive_set: bool,

  is_mouse_in: bool,
  is_mouse_down_in_history: bool,

  on_mouse_click: Option<Box<dyn FnMut(&mut DynCx, HitPoint3D)>>,
  on_mouse_hovering: Option<Box<dyn FnMut(&mut DynCx, HitPoint3D)>>,
  on_mouse_in: Option<Box<dyn FnMut(&mut DynCx)>>,
  on_mouse_out: Option<Box<dyn FnMut(&mut DynCx)>>,
  on_mouse_down: Option<Box<dyn FnMut(&mut DynCx, HitPoint3D)>>,

  std_model: EntityHandle<StandardModelEntity>,
  model: EntityHandle<SceneModelEntity>,
  pub(crate) node: EntityHandle<SceneNodeEntity>,
  material: EntityHandle<UnlitMaterialEntity>,
  mesh: AttributesMeshEntities,
}

impl Widget for UIWidgetModel {
  fn update_view(&mut self, _: &mut DynCx) {}
  fn update_state(&mut self, cx: &mut DynCx) {
    access_cx_mut!(
      cx,
      sm_intersection_gp,
      WidgetSceneModelIntersectionGroupConfig
    );
    if self.mouse_interactive != self.registered_interactive_set {
      if self.mouse_interactive && !self.registered_interactive_set {
        sm_intersection_gp.group.insert(self.model);
      }
      if !self.mouse_interactive && self.registered_interactive_set {
        sm_intersection_gp.group.remove(&self.model);
      }
      self.registered_interactive_set = self.mouse_interactive;
    }

    access_cx!(cx, platform_event, PlatformEventInput);
    access_cx!(cx, interaction_cx, Interaction3dCtx);

    #[allow(unused_variables)]
    fn debug(label: &str) {
      // println!("{}", label);
    }

    if platform_event.window_state.has_any_mouse_event
      && self.mouse_interactive
      && self.has_any_mouse_event_handler()
    {
      let is_pressing = platform_event.state_delta.is_left_mouse_pressing();
      let is_releasing = platform_event.state_delta.is_left_mouse_releasing();

      let mut current_frame_hitting = None;
      if let Some((hit, model)) = interaction_cx.world_ray_intersected_nearest {
        current_frame_hitting = (model == self.model).then_some(hit);
      }

      if let Some(hitting) = current_frame_hitting {
        if !self.is_mouse_in {
          debug("mouse in");
          self.is_mouse_in = true;
          if let Some(on_mouse_in) = &mut self.on_mouse_in {
            on_mouse_in(cx);
          }
        }
        debug("mouse hovering");
        if let Some(on_mouse_hovering) = &mut self.on_mouse_hovering {
          on_mouse_hovering(cx, hitting);
        }
        if is_pressing {
          debug("mouse down");
          if let Some(on_mouse_down) = &mut self.on_mouse_down {
            on_mouse_down(cx, current_frame_hitting.unwrap());
          }
          self.is_mouse_down_in_history = true;
        }
        if is_releasing && self.is_mouse_down_in_history {
          debug("click");
          if let Some(on_mouse_click) = &mut self.on_mouse_click {
            on_mouse_click(cx, current_frame_hitting.unwrap());
          }
          self.is_mouse_down_in_history = false;
        }
      } else if self.is_mouse_in {
        debug("mouse out");
        self.is_mouse_in = false;
        if let Some(on_mouse_out) = &mut self.on_mouse_out {
          on_mouse_out(cx);
        }
      }
    }
  }
  fn clean_up(&mut self, cx: &mut DynCx) {
    access_cx_mut!(cx, scene_cx, SceneWriter);
    self.do_cleanup(scene_cx);
  }
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
      mouse_interactive: true,
      registered_interactive_set: false,
      is_mouse_in: false,
      is_mouse_down_in_history: false,
      on_mouse_click: None,
      on_mouse_hovering: None,
      on_mouse_in: None,
      on_mouse_out: None,
      on_mouse_down: None,
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

  fn has_any_mouse_event_handler(&self) -> bool {
    self.on_mouse_click.is_some()
      || self.on_mouse_hovering.is_some()
      || self.on_mouse_down.is_some()
  }

  pub fn set_mouse_interactive(&mut self, v: bool) -> &mut Self {
    self.mouse_interactive = v;
    if !self.mouse_interactive {
      self.is_mouse_in = false;
      self.is_mouse_down_in_history = false
    }
    self
  }

  pub fn with_on_mouse_click(mut self, f: impl FnMut(&mut DynCx, HitPoint3D) + 'static) -> Self {
    self.on_mouse_click = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_in(mut self, f: impl FnMut(&mut DynCx) + 'static) -> Self {
    self.on_mouse_in = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_out(mut self, f: impl FnMut(&mut DynCx) + 'static) -> Self {
    self.on_mouse_out = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_hovering(mut self, f: impl FnMut(&mut DynCx, HitPoint3D) + 'static) -> Self {
    self.on_mouse_hovering = Some(Box::new(f));
    self
  }
  pub fn with_on_mouse_down(mut self, f: impl FnMut(&mut DynCx, HitPoint3D) + 'static) -> Self {
    self.on_mouse_down = Some(Box::new(f));
    self
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
