pub struct Gizmo {
  scale: AxisScaleGizmo,
  rotation: RotationGizmo,
  translate: MovingGizmo,
}

pub struct AxisScaleGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_box() -> Box<dyn SceneRenderable> {
  let mesh = CubeMeshParameter::default().tessellate();
  let mesh = MeshCell::new(MeshSource::new(mesh));
  todo!();
}

pub struct RotationGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_rotation_circle() -> Box<dyn SceneRenderable> {
  let position = Vec::new();
  for i in 0..50 {
    //
    position.push(Vec3::new())
  }
  todo!();
}

pub struct MovingGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_axis_arrow() -> Box<dyn SceneRenderable> {
  todo!();
}
