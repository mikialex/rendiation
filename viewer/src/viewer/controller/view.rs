use std::{
  pin::Pin,
  task::{Context, Poll},
};

use futures::Stream;
use incremental::ApplicableIncremental;
use interphaser::{
  mouse, mouse_move,
  winit::event::{ElementState, Event, MouseButton},
  CanvasWindowPositionInfo, WindowState,
};
use rendiation_algebra::Vec3;
use rendiation_geometry::OptionalNearest;
use rendiation_mesh_core::MeshBufferHitPoint;
use rendiation_scene_interaction::*;
use webgpu::{FrameRenderPass, RenderComponentAny};

use crate::*;

pub enum ViewReaction<V, T: ApplicableIncremental> {
  /// emit self special event
  ViewEvent(V),
  /// do state mutation
  StateDelta(T::Delta),
}

#[derive(Clone, Copy)]
pub enum Event3D {
  MouseDown { world_position: Vec3<f32> },
  MouseMove { world_position: Vec3<f32> },
  MouseUp { world_position: Vec3<f32> },
}

pub enum ViewRequest3D<'a, 'b, 'c> {
  Event(&'a mut EventCtx3D<'b>),
  Render {
    pass: &'a mut FrameRenderPass<'b, 'c>,
    dispatcher: &'a dyn RenderComponentAny,
    camera: &'a SceneCamera,
    scene: &'a SceneRenderResourceGroup<'b>,
  },
  HitTest {
    ctx: &'a SceneRayInteractiveCtx<'a>,
    hit_world_position: &'a mut OptionalNearest<MeshBufferHitPoint>,
  },
}

pub trait View3D: Stream<Item = ()> + Unpin {
  fn request(&mut self, detail: &mut ViewRequest3D);
}

pub struct EventCtx3D<'a> {
  pub window_states: &'a WindowState,
  pub raw_event: &'a Event<'a, ()>,
  pub info: &'a CanvasWindowPositionInfo,
  pub scene: &'a SceneCoreImpl,

  pub node_sys: &'a SceneNodeDeriveSystem,
  pub interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
}

impl<'a> EventCtx3D<'a> {
  pub fn new(
    window_states: &'a WindowState,
    raw_event: &'a Event<'a, ()>,
    info: &'a CanvasWindowPositionInfo,
    scene: &'a SceneCoreImpl,
    interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
    node_sys: &'a SceneNodeDeriveSystem,
  ) -> Self {
    Self {
      window_states,
      raw_event,
      info,
      scene,
      interactive_ctx,
      node_sys,
    }
  }
}

pub struct ViewGroup3D {
  collection: Vec<Box<dyn View3D>>,
}

impl ViewGroup3D {
  #[must_use]
  pub fn with(mut self, item: impl View3D + 'static) -> Self {
    self.collection.push(Box::new(item));
    self
  }
}

pub fn collection3d() -> ViewGroup3D {
  ViewGroup3D {
    collection: Default::default(),
  }
}

impl Stream for ViewGroup3D {
  type Item = ();

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    todo!()
  }
}

pub struct View3dEventWrap<T, F> {
  inner: T,
  f: F,
}
impl<T, F> Stream for View3dEventWrap<T, F>
where
  T: Unpin,
  T: Stream<Item = ()>,
  T: Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Pin::new(&mut self.inner).poll_next(cx)
  }
}
impl<T: View3D, F> View3D for View3dEventWrap<T, F>
where
  Self: Stream<Item = ()> + Unpin,
{
  fn request(&mut self, detail: &mut ViewRequest3D) {
    self.inner.request(detail)
  }
}

impl<C: View3D, X> View3D for interphaser::ReactiveNestedView<C, X>
where
  Self: Stream<Item = ()> + Unpin,
{
  fn request(&mut self, detail: &mut ViewRequest3D) {
    self.inner.request(detail)
  }
}

impl SceneRayInteractive for dyn View3D {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    let mut r = Default::default();
    self.request(&mut ViewRequest3D::HitTest {
      ctx,
      hit_world_position: &mut r,
    });
    r
  }
}

impl SceneRenderable for dyn View3D {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    self.request(&mut ViewRequest3D::Render {
      pass,
      dispatcher,
      camera,
      scene,
    })
  }
}

impl View3D for ViewGroup3D {
  fn request(&mut self, detail: &mut ViewRequest3D) {
    match detail {
      ViewRequest3D::Event(e) => {
        for c in &self.collection {
          c.request(detail)
        }
      }
      ViewRequest3D::Render {
        pass,
        dispatcher,
        camera,
        scene,
      } => {
        for c in &self.collection {
          c.request(&mut ViewRequest3D::Render {
            pass,
            dispatcher,
            camera,
            scene,
          });
        }
      }
      ViewRequest3D::HitTest {
        ctx,
        hit_world_position,
      } => {
        for c in &self.collection {
          c.request(&mut ViewRequest3D::HitTest {
            ctx,
            hit_world_position,
          })
        }
      }
    }
  }
}

pub fn map_3d_event(hit: MeshBufferHitPoint, event: &Event<()>) -> Option<Event3D> {
  if mouse_move(event).is_some() {
    Event3D::MouseMove {
      world_position: hit.hit.position,
    }
    .into()
  } else if let Some((button, state)) = mouse(event) {
    if button == MouseButton::Left {
      let e = match state {
        ElementState::Pressed => Event3D::MouseDown {
          world_position: hit.hit.position,
        },
        ElementState::Released => Event3D::MouseUp {
          world_position: hit.hit.position,
        },
      };
      Some(e)
    } else {
      None
    }
  } else {
    None
  }
}
