pub struct AxisHelper {
  pub enabled: bool,
}

impl AxisHelper {
  pub fn new(scene: &SceneContainer) -> Self {
    todo!()
  }
}

pub struct SceneContainer {
  scene: Rc<RefCell<Scene>>,
}

pub struct SceneContent<T> {
  item: T,
  scene: Weak<Scene>,
}
