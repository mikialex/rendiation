use std::any::Any;

use rendiation_algebra::*;
use rendiation_geometry::{Box3, Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig};

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

type Listener = Box<dyn Fn(&mut dyn Any, &dyn Any)>;

pub struct InteractiveWatchable<T> {
  inner: T,
  callbacks: Vec<Listener>,
}

impl<T> InteractiveWatchable<T> {
  pub fn on<S: 'static, E: 'static>(mut self, cb: impl Fn(&mut S, &E) + 'static) -> Self {
    self.callbacks.push(Box::new(move |state, event| {
      if let Some(state) = state.downcast_mut::<S>() {
        if let Some(event) = event.downcast_ref::<E>() {
          cb(state, event)
        }
      }
    }));
    self
  }
}

pub trait InteractiveWatchableInit<T> {
  fn eventable(self) -> InteractiveWatchable<T>;
}

impl<T: SceneRenderable> InteractiveWatchableInit<T> for T {
  fn eventable(self) -> InteractiveWatchable<T> {
    InteractiveWatchable {
      inner: self,
      callbacks: Default::default(),
    }
  }
}

impl<T: SceneRenderable> Component3D for InteractiveWatchable<T> {
  fn event(&self, event: &dyn Any, states: &mut dyn Any) {
    for cb in &self.callbacks {
      cb(states, event)
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
}
