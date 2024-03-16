use bytemuck::{Pod, Zeroable};
use rendiation_shader_api::*;

use crate::*;

pub struct TriangulationBasedPainter<R> {
  recording: GraphicsRepresentation,
  transform_stack: Vec<TransformState>,
  masking_stack: Vec<GraphicsRepresentation>,
  filter_stack: Vec<CanvasEffect>,
  renderer: R,
}

pub trait TriangulationBasedRendererImpl {
  type Image;
  fn render(&mut self, target: &Self::Image, content: &GraphicsRepresentation);
}

only_vertex!(GeometryColorWithAlphaPremultiplied, Vec3<f32>);
only_vertex!(UIMetadata, u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, ShaderStruct, ShaderVertex, Zeroable, Pod)]
pub struct GraphicsVertex {
  #[semantic(GeometryPosition2D)]
  pub position: Vec2<f32>,
  #[semantic(GeometryColorWithAlphaPremultiplied)]
  pub color: Vec3<f32>,
  #[semantic(GeometryUV)]
  pub uv: Vec2<f32>,
  #[semantic(UIMetadata)]
  pub object_id: u32, // point to ObjectMetaData[]
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct)]
pub struct ObjectMetaData {
  pub world_transform: Shader16PaddedMat3,
  pub uv_offset: Vec2<f32>,
  pub uv_scale: Vec2<f32>,
  /// fot this id, 0 means not image, 1 means text atlas, so the real image index is id - 1
  pub image_id: u32,
}

struct TransformState {
  local: Mat3<f32>,
  world_computed: Mat3<f32>,
}

#[derive(Default)]
pub struct GraphicsRepresentation {
  pub object_meta: Vec<ObjectMetaData>,
  pub vertices: Vec<GraphicsVertex>,
  pub indices: Vec<u32>,

  pub images: Vec<GraphicsImageData>,
}

impl GraphicsRepresentation {
  pub fn reset(&mut self) {
    self.object_meta.clear();
    self.vertices.clear();
    self.indices.clear();
    self.images.clear();
  }
}

impl<R> TriangulationBasedPainter<R> {
  fn get_current_world_transform(&self) -> Mat3<f32> {
    self
      .transform_stack
      .last()
      .map(|v| v.world_computed)
      .unwrap_or(Mat3::identity())
  }
}

pub enum GraphicsImageData {
  Bitmap,
  Gpu,
}

impl<R> PainterAPI for TriangulationBasedPainter<R>
where
  R: TriangulationBasedRendererImpl<Image = GraphicsImageData>,
{
  fn reset(&mut self) {
    self.recording.reset();
    self.transform_stack.clear();
    self.masking_stack.clear();
    self.filter_stack.clear();
  }

  type Image = GraphicsImageData;

  fn register_image(&mut self, image: Self::Image) -> TextureHandle {
    let handle = self.recording.images.len();
    self.recording.images.push(image);
    handle
  }

  /// We are not using the raw path data as the baked data, this will cause the visual
  /// degradation we do not care because the triangulation is not view dependent by design for
  /// performance reason.
  type Baked = GraphicsRepresentation;

  fn draw_bake(&mut self, _: &Self::Baked) {
    todo!()
  }

  fn bake(self) -> Self::Baked {
    self.recording
  }
  fn render(&mut self, target: &Self::Image) {
    self.renderer.render(target, &self.recording)
  }

  fn stroke_shape(&mut self, shape: &Shape, style: &StrokeStyle) {
    let world_transform = self.get_current_world_transform().into();
    let meta = ObjectMetaData {
      world_transform,
      uv_offset: Default::default(),
      uv_scale: Default::default(),
      image_id: 0,
      ..Zeroable::zeroed()
    };

    let meta_index = self.recording.object_meta.len();
    self.recording.object_meta.push(meta);

    let builder = MeshBuilder::new(
      &mut self.recording.vertices,
      &mut self.recording.indices,
      meta_index,
    );
    triangulate_stroke(shape, style, builder);
  }

  fn fill_shape(&mut self, shape: &Shape, style: &FillStyle) {
    let world_transform = self.get_current_world_transform().into();
    let meta = ObjectMetaData {
      world_transform,
      uv_offset: Default::default(),
      uv_scale: Default::default(),
      image_id: 0,
      ..Zeroable::zeroed()
    };
    let meta_index = self.recording.object_meta.len();
    self.recording.object_meta.push(meta);

    let builder = MeshBuilder::new(
      &mut self.recording.vertices,
      &mut self.recording.indices,
      meta_index,
    );
    triangulate_fill(shape, style, builder);
  }

  fn push_transform(&mut self, transform: Mat3<f32>) {
    let world_computed = transform * self.get_current_world_transform();
    self.transform_stack.push(TransformState {
      local: transform,
      world_computed,
    })
  }

  fn pop_transform(&mut self) -> Option<Mat3<f32>> {
    self.transform_stack.pop().map(|v| v.local)
  }

  fn push_mask(&mut self, mask: Self::Baked) {
    self.masking_stack.push(mask)
  }

  fn pop_mask(&mut self) -> Option<Self::Baked> {
    self.masking_stack.pop()
  }

  fn push_filter(&mut self, effect: CanvasEffect) {
    self.filter_stack.push(effect)
  }

  fn pop_filter(&mut self) -> Option<CanvasEffect> {
    self.filter_stack.pop()
  }
}

// todo, handle max u32
struct MeshBuilder<'a> {
  vertices: &'a mut Vec<GraphicsVertex>,
  indices: &'a mut Vec<u32>,
  meta_index: u32,
}

impl<'a> MeshBuilder<'a> {
  pub fn new(
    vertices: &'a mut Vec<GraphicsVertex>,
    indices: &'a mut Vec<u32>,
    meta_index: usize,
  ) -> Self {
    Self {
      vertices,
      indices,
      meta_index: meta_index as u32,
    }
  }
}

impl<'a> lyon::lyon_tessellation::GeometryBuilder for MeshBuilder<'a> {
  fn add_triangle(
    &mut self,
    a: lyon::lyon_tessellation::VertexId,
    b: lyon::lyon_tessellation::VertexId,
    c: lyon::lyon_tessellation::VertexId,
  ) {
    self.indices.push(a.offset());
    self.indices.push(b.offset());
    self.indices.push(c.offset());
  }
}

impl<'a> lyon::lyon_tessellation::FillGeometryBuilder for MeshBuilder<'a> {
  fn add_fill_vertex(
    &mut self,
    vertex: lyon::lyon_tessellation::FillVertex,
  ) -> Result<lyon::lyon_tessellation::VertexId, lyon::lyon_tessellation::GeometryBuilderError> {
    // todo uv color
    let position = vertex.position();
    let vertex = GraphicsVertex {
      position: Vec2::new(position.x, position.y),
      color: Vec3::zero(),
      uv: Vec2::zero(),
      object_id: self.meta_index,
    };
    let index = self.vertices.len();
    self.vertices.push(vertex);
    Ok(lyon::lyon_tessellation::VertexId::from(index as u32))
  }
}

impl<'a> lyon::lyon_tessellation::StrokeGeometryBuilder for MeshBuilder<'a> {
  fn add_stroke_vertex(
    &mut self,
    vertex: lyon::lyon_tessellation::StrokeVertex,
  ) -> Result<lyon::lyon_tessellation::VertexId, lyon::lyon_tessellation::GeometryBuilderError> {
    // todo uv color
    let position = vertex.position();
    let vertex = GraphicsVertex {
      position: Vec2::new(position.x, position.y),
      color: Vec3::zero(),
      uv: Vec2::zero(),
      object_id: self.meta_index,
    };
    let index = self.vertices.len();
    self.vertices.push(vertex);
    Ok(lyon::lyon_tessellation::VertexId::from(index as u32))
  }
}

fn triangulate_stroke(
  shape: &Shape,
  _style: &StrokeStyle,
  mut builder: impl lyon::lyon_tessellation::StrokeGeometryBuilder,
) {
  use lyon::tessellation::{StrokeOptions, StrokeTessellator};

  let options = StrokeOptions::tolerance(0.1);
  let mut tessellator = StrokeTessellator::new();

  let mut builder = tessellator.builder(&options, &mut builder);

  match shape {
    Shape::Rect(rect) => {
      builder.add_rectangle(&into_lyon_rect(rect), lyon::path::Winding::Positive)
    }
    Shape::RoundCorneredRect(round_rect) => builder.add_rounded_rectangle(
      &into_lyon_rect(&round_rect.rect),
      &into_lyon_radius(&round_rect.radius),
      lyon::path::Winding::Positive,
    ),
    Shape::Path(path) => {
      for seg in &path.sub_paths {
        builder.begin(into_lyon_point(seg.start));
        for p in &seg.paths {
          match &p.path {
            Path2dType::Line(_) => {
              builder.line_to(into_lyon_point(p.end_point));
            }
            Path2dType::QuadraticBezier(c) => {
              builder.quadratic_bezier_to(into_lyon_point(c.ctrl), into_lyon_point(p.end_point));
            }
            Path2dType::CubicBezier(c) => {
              builder.cubic_bezier_to(
                into_lyon_point(c.ctrl1),
                into_lyon_point(c.ctrl2),
                into_lyon_point(p.end_point),
              );
            }
          }
        }
        builder.end(seg.closed)
      }
    }
  }

  builder.build().unwrap();
}

fn triangulate_fill(
  shape: &Shape,
  _style: &FillStyle,
  mut builder: impl lyon::lyon_tessellation::FillGeometryBuilder,
) {
  use lyon::tessellation::{FillOptions, FillTessellator};

  let options = FillOptions::tolerance(0.1);
  let mut tessellator = FillTessellator::new();

  let mut builder = tessellator.builder(&options, &mut builder);

  match shape {
    Shape::Rect(rect) => {
      builder.add_rectangle(&into_lyon_rect(rect), lyon::path::Winding::Positive)
    }
    Shape::RoundCorneredRect(round_rect) => builder.add_rounded_rectangle(
      &into_lyon_rect(&round_rect.rect),
      &into_lyon_radius(&round_rect.radius),
      lyon::path::Winding::Positive,
    ),
    Shape::Path(path) => {
      for seg in &path.sub_paths {
        builder.begin(into_lyon_point(seg.start));
        for p in &seg.paths {
          match &p.path {
            Path2dType::Line(_) => {
              builder.line_to(into_lyon_point(p.end_point));
            }
            Path2dType::QuadraticBezier(c) => {
              builder.quadratic_bezier_to(into_lyon_point(c.ctrl), into_lyon_point(p.end_point));
            }
            Path2dType::CubicBezier(c) => {
              builder.cubic_bezier_to(
                into_lyon_point(c.ctrl1),
                into_lyon_point(c.ctrl2),
                into_lyon_point(p.end_point),
              );
            }
          }
        }
        builder.end(seg.closed)
      }
    }
  }

  builder.build().unwrap();
}

fn into_lyon_point(v: Vec2<f32>) -> lyon::math::Point {
  lyon::math::point(v.x, v.y)
}

fn into_lyon_rect(rect: &RectangleShape) -> lyon::math::Box2D {
  lyon::math::Box2D {
    min: lyon::math::point(rect.x, rect.y),
    max: lyon::math::point(rect.x + rect.width, rect.y + rect.height),
  }
}

fn into_lyon_radius(radius: &RadiusGroup) -> lyon::path::builder::BorderRadii {
  lyon::path::builder::BorderRadii {
    top_left: radius.top_left,
    top_right: radius.top_right,
    bottom_left: radius.bottom_left,
    bottom_right: radius.bottom_right,
  }
}
