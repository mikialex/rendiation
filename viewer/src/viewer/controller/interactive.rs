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

type Listener<S> = Box<dyn Fn(&mut S, &dyn Any)>;

pub struct InteractiveWatchable<T, S> {
  inner: T,
  callbacks: Vec<Listener<S>>,
  updates: Option<Box<dyn Fn(&S, &mut T)>>,
}

impl<T, S> InteractiveWatchable<T, S> {
  pub fn on<E: 'static>(mut self, cb: impl Fn(&mut S, &E) + 'static) -> Self {
    self.callbacks.push(Box::new(move |state, event| {
      if let Some(event) = event.downcast_ref::<E>() {
        cb(state, event)
      }
    }));
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

impl<T: SceneRenderable, S> Component3D<S> for InteractiveWatchable<T, S> {
  fn event(&self, event: &dyn Any, states: &mut S) {
    for cb in &self.callbacks {
      cb(states, event)
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
