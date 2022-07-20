use interphaser::Component;
use rendiation_algebra::*;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig};

use crate::*;

pub enum Event3D {
  MouseDown { world_position: Vec3<f32> },
  MouseMove { world_position: Vec3<f32> },
  MouseUp { world_position: Vec3<f32> },
}

type Listener<S> = Box<dyn Fn(&mut S, &Event3D)>;

pub struct InteractiveWatchable<T, S> {
  inner: T,
  callbacks: Vec<Listener<S>>,
  updates: Option<Box<dyn Fn(&S, &mut T)>>,
}

impl<T, S> InteractiveWatchable<T, S> {
  pub fn on(mut self, cb: impl Fn(&mut S, &Event3D) + 'static) -> Self {
    self
      .callbacks
      .push(Box::new(move |state, event| cb(state, event)));
    self
  }
  pub fn update(mut self, updater: impl Fn(&S, &mut T) + 'static) -> Self {
    self.updates = Some(Box::new(updater));
    self
  }
}

pub trait InteractiveWatchableInit<T> {
  fn eventable<S>(self) -> InteractiveWatchable<T, S>;
}

impl<T: SceneRenderable> InteractiveWatchableInit<T> for T {
  fn eventable<S>(self) -> InteractiveWatchable<T, S> {
    InteractiveWatchable {
      inner: self,
      callbacks: Default::default(),
      updates: None,
    }
  }
}

impl<T: SceneRenderable, S> Component<S, System3D> for InteractiveWatchable<T, S> {
  fn event(&mut self, states: &mut S, event: &mut EventCtx3D) {
    for cb in &self.callbacks {
      if let Some(event_3d) = &event.event_3d {
        cb(states, event_3d)
      }
    }
  }
}

impl<T: SceneRenderable, S> SceneRenderable for InteractiveWatchable<T, S> {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.inner.render(pass, dispatcher, camera)
  }
}

impl<T: SceneRayInteractive, S> SceneRayInteractive for InteractiveWatchable<T, S> {
  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    None
  }
}
