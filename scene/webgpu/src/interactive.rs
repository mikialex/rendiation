use crate::*;

pub struct MouseDown3DEvent {
  pub world_position: Vec3<f32>,
}
pub struct MouseMove3DEvent {
  pub world_position: Vec3<f32>,
}
pub struct MouseUp3DEvent {
  pub world_position: Vec3<f32>,
}

type Listener<T> = Box<dyn Fn(&T, &dyn Any)>;

pub struct InteractiveWatchable<T> {
  inner: T,
  callbacks: Vec<Listener<T>>,
}

impl<T> InteractiveWatchable<T> {
  pub fn on(&mut self, cb: impl Fn(&T, &dyn Any) + 'static) -> &mut Self {
    self.callbacks.push(Box::new(cb));
    self
  }
}

pub trait InteractiveWatchableInit<T> {
  fn interactive_watchable(self) -> InteractiveWatchable<T>;
}

impl<T: SceneRenderable> InteractiveWatchableInit<T> for T {
  fn interactive_watchable(self) -> InteractiveWatchable<T> {
    InteractiveWatchable {
      inner: self,
      callbacks: Default::default(),
    }
  }
}

impl<T: SceneRenderable> SceneRenderable for InteractiveWatchable<T> {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.inner.render(pass, dispatcher, camera)
  }

  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    None
  }

  fn get_bounding_info(&self) -> Option<Box3> {
    self.inner.get_bounding_info()
  }

  fn event(&mut self, event: &dyn Any) {
    for cb in &mut self.callbacks {
      cb(&self.inner, event)
    }
  }
}
