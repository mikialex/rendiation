use rendiation_infinity_primitive::*;

use crate::*;

pub struct GridGround<'a> {
  pub shading: &'a UniformBufferCachedDataView<GridEffect>,
  pub plane: &'a UniformBufferCachedDataView<ShaderPlaneUniform>,
  pub reversed_depth: bool,
  pub camera: &'a dyn RenderComponent,
}

impl PassContent for GridGround<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth);

    let plane = InfinityShaderPlaneEffect {
      plane: self.plane,
      camera: self.camera,
      reversed_depth: self.reversed_depth,
    };

    let grid = GridGroundShading {
      shading: self.shading,
      reversed_depth: self.reversed_depth,
    };

    let com: [&dyn RenderComponent; 3] = [&base, &plane, &grid];
    let com = RenderArray(com);

    com.render(&mut pass.ctx, RenderMethod::TraditionalDraw(PLANE_DRAW_CMD))
  }
}

#[repr(C)]
#[shader_struct(std140)]
#[derive(Copy, Clone, PartialEq)]
pub struct GridEffect {
  /// grid cell count per world unit, power of two value is recommended for best precision
  pub scale: Vec2<f32>,
  /// line width relative to the cell size, in range 0 to 1. if the screen space line width is
  /// enabled, this is the max line width, see `pristine_grid` for details
  pub line_width: Vec2<f32>,
  /// line color, the alpha is multiplied by the line coverage
  pub color: Vec4<f32>,
  /// line width in pixel, only used if the screen space line width is enabled
  pub line_pixel_width: f32,
  pub screen_space_line_width: Bool,
}

impl Default for GridEffect {
  fn default() -> Self {
    Self {
      scale: Vec2::one(),
      line_width: Vec2::splat(0.1),
      color: Vec4::new(0.2, 0.2, 0.2, 1.0),
      line_pixel_width: 1.5,
      screen_space_line_width: true.into(),
      ..Zeroable::zeroed()
    }
  }
}

pub struct GridGroundShading<'a> {
  pub shading: &'a UniformBufferCachedDataView<GridEffect>,
  pub reversed_depth: bool, // this should have already been hashed in infinity shader
}
impl ShaderHashProvider for GridGroundShading<'_> {
  shader_hash_type_id! {GridGroundShading<'static>}
}
impl ShaderPassBuilder for GridGroundShading<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.shading);
  }
}
impl GraphicsShaderProvider for GridGroundShading<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let shading = binding.bind_by(&self.shading).load().expand();
      let render_position = builder.query::<FragmentRenderPosition>();
      let camera_world_position = builder.query::<CameraWorldPositionHP>().expand();

      // the grid only cares about the fractional part of the grid coordinate. adding the camera
      // world position to the render position directly loses precision per pixel when far from
      // origin, which makes the lines jagged and flicker. instead we take the fract of each high
      // precision part separately. the rounding error of these terms only depends on the camera,
      // so it is the same for all pixels and at most shifts the entire grid slightly (no error at
      // all if the scale is a power of two).
      let local = render_position.xz() * shading.scale;
      let camera_offset = (camera_world_position.f1.xz() * shading.scale).fract()
        + (camera_world_position.f2.xz() * shading.scale).fract();

      let coverage = pristine_grid_fn(
        camera_offset + local,
        local,
        shading.line_width,
        shading.line_pixel_width,
        shading.screen_space_line_width.into_bool(),
      );

      let output: Node<Vec4<f32>> = (shading.color.xyz(), shading.color.w() * coverage).into();
      builder.register::<DefaultDisplay>(output);

      let state = RasterizationStates {
        blend: BlendState::ALPHA_BLENDING.into(),
        depth_write_enabled: false,
        ..Default::default()
      };
      apply_pipeline_frag_builder(&state, self.reversed_depth, builder);
    })
  }
}

/// Anti aliased grid line coverage, the line is at the integer coordinate.
///
/// The implementation is based on "The Best Darn Grid Shader (Yet)" by Ben Golus:
/// https://bgolus.medium.com/the-best-darn-grid-shader-yet-727f9278b9d8
///
/// Compared with the common `abs(fract(x - 0.5) - 0.5) / fwidth(x)` approach, the line width is
/// defined in grid space so it has correct perspective. When the line is thinner than one pixel,
/// it is drawn at one pixel width with the coverage scaled down (like the phone wire AA), and when
/// one pixel covers a full cell, the result fades to the average coverage of the cell. This avoids
/// the moire pattern and the grid turning into a solid color at the horizon.
///
/// If `screen_space` is true, the line width is `line_pixel_width` pixels, but limited by the
/// `line_width` in grid space. A pure screen space width line will cover the entire cell at
/// distance and the grid becomes solid color, with the limit, the near lines keep the constant
/// pixel width, and the far lines switch to the grid space width smoothly, then the above fading
/// works as usual.
///
/// `coord_for_derivative` must be continuous across pixels and have the same derivative as `coord`,
/// it is used to compute the screen space derivative because the `coord` may be offset by a fract
/// value which has discontinuity.
#[shader_fn]
fn pristine_grid(
  coord: Node<Vec2<f32>>,
  coord_for_derivative: Node<Vec2<f32>>,
  line_width: Node<Vec2<f32>>,
  line_pixel_width: Node<f32>,
  screen_space: Node<bool>,
) -> Node<f32> {
  // per axis derivative length, fwidth (abs(dx) + abs(dy)) overestimates it up to sqrt(2) times
  let dx = coord_for_derivative.dpdx();
  let dy = coord_for_derivative.dpdy();
  let uv_deriv = (dx * dx + dy * dy).sqrt();

  let line_width = screen_space
    .select(line_width.min(uv_deriv * line_pixel_width), line_width)
    .saturate();

  // when the line is wider than half cell, draw the inverted thin line for better quality
  let invert_line = line_width.greater_than(val(Vec2::splat(0.5)));
  let target_width = invert_line.select_per_channel(val(1.0) - line_width, line_width);
  let draw_width = target_width.clamp(uv_deriv, val(Vec2::splat(0.5)));
  let line_aa = uv_deriv.max(val(Vec2::splat(0.000001))) * val(1.5);

  // 1 at the line and 0 at the cell center, flipped to be the distance to the line (in unit of
  // half cell) for the non inverted case
  let grid_uv = (coord.fract() * val(2.0) - val(1.0)).abs();
  let grid_uv = invert_line.select_per_channel(grid_uv, val(1.0) - grid_uv);

  let grid = grid_uv.smoothstep_per_channel(draw_width + line_aa, draw_width - line_aa);
  // the line is drawn at least one pixel wide, so fade it by the real width
  let grid = grid * (target_width / draw_width).saturate();
  // fade to the average coverage when the cell is smaller than one pixel
  let grid = (uv_deriv * val(2.0) - val(1.0))
    .saturate()
    .mix_per_channel(grid, target_width);
  let grid = invert_line.select_per_channel(val(1.0) - grid, grid);

  // combine the two axes by coverage union, max or min would make the crossing too dark or bright
  grid.y().mix(grid.x(), val(1.0))
}
