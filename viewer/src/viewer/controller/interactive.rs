use incremental::Incremental;
use rendiation_algebra::*;
use rendiation_geometry::OptionalNearest;
use rendiation_renderable_mesh::mesh::MeshBufferHitPoint;

use crate::*;

pub enum ViewReaction<V, T: Incremental> {
  /// emit self special event
  ViewEvent(V),
  /// do state mutation
  StateDelta(T::Delta),
}

/// View type could generic over any state T, as long as the T could provide
/// given logic for view type
pub trait View<T>
where
  T: Incremental,
{
  /// View type's own event type
  type Event;

  /// In event loop handling, the view type received platform event such as mouse move keyboard events,
  /// and decide should reactive to it or not, if so, mutate the model or emit
  /// the self::Event for further outer side handling. see ViewDelta.
  ///
  /// all mutation to the model should record delta by call cb passed from caller.
  ///
  /// In View hierarchy, event's mutation to state will pop up to the root, wrap the mutation to
  /// parent state's delta type. and in update logic, consumed from the root
  fn event(
    &mut self,
    model: &mut T,
    event: &mut EventCtx3D,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
  );

  /// update is responsible for map the state delta to to view property change
  /// the model here is the unmodified.
  fn update(&mut self, model: &T, delta: &T::Delta);
}

#[derive(Clone, Copy)]
pub enum Event3D {
  MouseDown { world_position: Vec3<f32> },
  MouseMove { world_position: Vec3<f32> },
  MouseUp { world_position: Vec3<f32> },
}

pub struct InteractiveWatchable<T, S: Incremental> {
  inner: T,
  callbacks: Vec<Box<dyn FnMut(&mut S, &EventCtx3D, &mut dyn FnMut(S::Delta))>>,
  updates: Option<Box<dyn FnMut(DeltaView<S>, &mut T)>>,
}

impl<T, S: Incremental> InteractiveWatchable<T, S> {
  pub fn on(
    mut self,
    mut cb: impl FnMut(&mut S, &EventCtx3D, &mut dyn FnMut(S::Delta)) + 'static,
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
  fn eventable<S: Incremental>(self) -> InteractiveWatchable<T, S>;
}

impl<T: SceneRenderable> InteractiveWatchableInit<T> for T {
  fn eventable<S: Incremental>(self) -> InteractiveWatchable<T, S> {
    InteractiveWatchable {
      inner: self,
      callbacks: Default::default(),
      updates: None,
    }
  }
}

impl<T: SceneRenderable, S: Incremental> View<S> for InteractiveWatchable<T, S> {
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

impl<T: SceneRenderable, S: Incremental> SceneRenderable for InteractiveWatchable<T, S> {
  fn is_transparent(&self) -> bool {
    self.inner.is_transparent()
  }

  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.inner.render(pass, dispatcher, camera)
  }
}

impl<T: SceneRayInteractive, S: Incremental> SceneRayInteractive for InteractiveWatchable<T, S> {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self.inner.ray_pick_nearest(ctx)
  }
}
