use incremental::*;
use rendiation_algebra::*;
use rendiation_geometry::OptionalNearest;
use rendiation_mesh_core::MeshBufferHitPoint;
use rendiation_scene_interaction::*;
use webgpu::{FrameRenderPass, RenderComponentAny};

use crate::*;

#[derive(Clone, Copy)]
pub enum Event3D {
  MouseDown { world_position: Vec3<f32> },
  MouseMove { world_position: Vec3<f32> },
  MouseUp { world_position: Vec3<f32> },
}

pub struct InteractiveWatchable<T, S: ApplicableIncremental> {
  inner: T,
  callbacks: Vec<Box<dyn FnMut(&mut S, &EventCtx3D, &mut dyn FnMut(S::Delta))>>,
  updates: Option<Box<dyn FnMut(DeltaView<S>, &mut T)>>,
}

impl<T, S: ApplicableIncremental> InteractiveWatchable<T, S> {
  pub fn on(
    mut self,
    cb: impl FnMut(&mut S, &EventCtx3D, &mut dyn FnMut(S::Delta)) + 'static,
  ) -> Self {
    self.callbacks.push(Box::new(cb));
    self
  }
  pub fn update(mut self, updater: impl FnMut(DeltaView<S>, &mut T) + 'static) -> Self {
    self.updates = Some(Box::new(updater));
    self
  }
}

pub trait InteractiveWatchableInit<T> {
  fn eventable<S: ApplicableIncremental>(self) -> InteractiveWatchable<T, S>;
}

impl<T: SceneRenderable> InteractiveWatchableInit<T> for T {
  fn eventable<S: ApplicableIncremental>(self) -> InteractiveWatchable<T, S> {
    InteractiveWatchable {
      inner: self,
      callbacks: Default::default(),
      updates: None,
    }
  }
}

impl<T: SceneRenderable, S: ApplicableIncremental> ViewBase<S> for InteractiveWatchable<T, S> {
  type Event = ();

  fn event(
    &mut self,
    model: &mut S,
    event: &mut EventCtx3D,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, S>),
  ) {
    for cb_e in &mut self.callbacks {
      cb_e(model, event, &mut |d| cb(ViewReaction::StateDelta(d)));
    }
  }

  /// update is responsible for map the state delta to to view property change
  /// the model here is the unmodified.
  fn update(&mut self, model: &S, delta: &S::Delta) {
    if let Some(update) = &mut self.updates {
      update(DeltaView { delta, data: model }, &mut self.inner)
    }
  }
}

impl<T: SceneRenderable, S: ApplicableIncremental> SceneRenderable for InteractiveWatchable<T, S> {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    self.inner.render(pass, dispatcher, camera, scene)
  }
}

impl<T: SceneRayInteractive, S: ApplicableIncremental> SceneRayInteractive
  for InteractiveWatchable<T, S>
{
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self.inner.ray_pick_nearest(ctx)
  }
}
