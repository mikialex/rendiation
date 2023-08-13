use std::task::Poll;

use __core::{pin::Pin, task::Context};
use bytemuck::Zeroable;
use futures::Stream;
use incremental::*;
use reactive::*;
use rendiation_shader_api::*;
use webgpu::*;

use crate::*;

#[repr(C)]
#[derive(Clone, Incremental)]
pub struct FatLineMaterial {
  pub width: f32,
  pub state: MaterialStates,
}

impl FatLineMaterial {
  pub fn new(width: f32) -> Self {
    Self {
      width,
      state: MaterialStates::helper_like(),
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FatlineMaterialUniform {
  pub width: f32,
}

type ReactiveFatlineMaterialGPUInner =
  impl AsRef<RenderComponentCell<FatlineMaterialGPU>> + Stream<Item = RenderComponentDeltaFlag>;

#[pin_project::pin_project]
pub struct ReactiveFatlineMaterialGPU {
  #[pin]
  inner: ReactiveFatlineMaterialGPUInner,
}

impl Stream for ReactiveFatlineMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.inner.poll_next(cx)
  }
}

impl ReactiveRenderComponentSource for ReactiveFatlineMaterialGPU {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.inner.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl WebGPUMaterial for FatLineMaterial {
  type ReactiveGPU = ReactiveFatlineMaterialGPU;

  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let uniform = FatlineMaterialUniform {
      width: source.read().width,
      ..Zeroable::zeroed()
    };
    let uniform = create_uniform(uniform, &ctx.gpu.device);

    let gpu = FatlineMaterialGPU { uniform };
    let state = RenderComponentCell::new(gpu);

    let weak_material = source.downgrade();
    let ctx = ctx.clone();

    let inner = source
      .single_listen_by::<()>(any_change_no_init)
      .fold_signal(state, move |_, state| {
        if let Some(m) = weak_material.upgrade() {
          let uniform = FatlineMaterialUniform {
            width: m.read().width,
            ..Zeroable::zeroed()
          };
          state.inner.uniform.set(uniform);
          state.inner.uniform.upload(&ctx.gpu.queue);
        }
        RenderComponentDeltaFlag::Content.into()
      });

    ReactiveFatlineMaterialGPU { inner }
  }

  fn is_transparent(&self) -> bool {
    false
  }
}

pub struct FatlineMaterialGPU {
  uniform: UniformBufferDataView<FatlineMaterialUniform>,
}

impl Stream for FatlineMaterialGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
}

impl ShaderHashProvider for FatlineMaterialGPU {}

impl ShaderPassBuilder for FatlineMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform);
  }
}

impl GraphicsShaderProvider for FatlineMaterialGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, binding| {
      let uv = builder.query::<GeometryUV>()?;
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>()?;
      let material = binding.bind_by(&self.uniform).expand();

      let vertex_position = fatline_vertex(
        builder.query::<CameraProjectionMatrix>()?,
        builder.query::<CameraViewMatrix>()?,
        builder.query::<WorldMatrix>()?,
        builder.query::<FatLineStart>()?,
        builder.query::<FatLineEnd>()?,
        builder.query::<GeometryPosition>()?,
        builder.query::<RenderBufferSize>()?,
        material.width,
      );

      builder.register::<ClipPosition>(vertex_position);
      builder.set_vertex_out::<FragmentUv>(uv);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color_with_alpha);
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      let uv = builder.query::<FragmentUv>()?;
      let color = builder.query::<FragmentColorAndAlpha>()?;

      if_by(discard_fatline_round_corner(uv), || {
        builder.discard();
      });

      builder.register::<DefaultDisplay>(color);
      Ok(())
    })
  }
}

#[allow(clippy::too_many_arguments)]
fn fatline_vertex(
  projection: Node<Mat4<f32>>,
  view: Node<Mat4<f32>>,
  world_matrix: Node<Mat4<f32>>,
  fatline_start: Node<Vec3<f32>>,
  fatline_end: Node<Vec3<f32>>,
  position: Node<Vec3<f32>>,
  view_size: Node<Vec2<f32>>,
  width: Node<f32>,
) -> Node<Vec4<f32>> {
  let fatline_start: Node<Vec4<_>> = (fatline_start, val(1.0)).into();
  let fatline_end: Node<Vec4<_>> = (fatline_end, val(1.0)).into();
  // camera space
  let start = view * world_matrix * fatline_start;
  let end = view * world_matrix * fatline_end;

  // // special case for perspective projection, and segments that terminate either in, or behind,
  // the camera plane // clearly the gpu firmware has a way of addressing this issue when
  // projecting into ndc space // but we need to perform ndc-space calculations in the shader, so
  // we must address this issue directly // perhaps there is a more elegant solution --
  // WestLangley bool perspective = ( projection[ 2 ][ 3 ] == - 1.0 ); // 4th entry in the 3rd
  // column if ( perspective ) {{
  //     if ( start.z < 0.0 && end.z >= 0.0 ) {{
  //         trimSegment( start, end );
  //     }} else if ( end.z < 0.0 && start.z >= 0.0 ) {{
  //         trimSegment( end, start );
  //     }}
  // }}

  let aspect = view_size.x() / view_size.y();

  // clip space
  let clip_start = projection * start;
  let clip_end = projection * end;

  // ndc space
  let ndc_start = clip_start.xy() / clip_start.w();
  let ndc_end = clip_end.xy() / clip_end.w();

  // direction
  let dir = ndc_end - ndc_start;

  // account for clip-space aspect ratio
  let dir: Node<Vec2<_>> = (dir.x() * aspect, dir.y()).into();
  let dir = dir.normalize();

  // perpendicular to dir
  let offset: Node<Vec2<_>> = (dir.y(), -dir.x()).into();

  // undo aspect ratio adjustment
  let dir: Node<Vec2<_>> = (dir.x() / aspect, dir.y()).into();
  let offset: Node<Vec2<_>> = (offset.x() / aspect, dir.y()).into();
  let offset = offset.mutable();

  // sign flip
  if_by(position.x().less_than(0.), || {
    offset.set(-offset.get());
  });

  // end caps
  if_by(position.y().less_than(0.), || {
    offset.set(offset.get() - dir);
  });

  if_by(position.y().greater_than(1.), || {
    offset.set(offset.get() + dir);
  });

  let mut offset = offset.get();

  // adjust for fatLineWidth
  offset *= width.splat();
  // adjust for clip-space to screen-space conversion // maybe resolution should be based on
  // viewport ...
  offset = offset / view_size.y();

  // select end
  let clip = position.y().less_than(0.5).select(clip_start, clip_end);

  // back to clip space
  offset = offset * clip.w();
  (clip.xy() + offset, clip.zw()).into()
}

fn discard_fatline_round_corner(uv: Node<Vec2<f32>>) -> Node<bool> {
  let a = uv.x();
  let b = uv.y() + uv.y().greater_than(0.).select(-1., 1.);
  let len2 = a * a + b * b;

  uv.y().abs().greater_than(1.).and(len2.greater_than(1.))
}
