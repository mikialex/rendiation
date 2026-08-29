use crate::*;

only_vertex!(GeometryPosition2D, Vec2<f32>);
only_vertex!(GeometryPosition, Vec3<f32>);
only_vertex!(GeometryNormal, Vec3<f32>);
// the forth channel is hand ness
only_vertex!(GeometryTangent, Vec4<f32>);

only_fragment!(AlphaChannel, f32);

pub type GeometryUV = GeometryUVChannel<0>;
pub struct GeometryUVChannel<const I: usize>;
impl<const I: usize> SemanticVertexShaderValue for GeometryUVChannel<I> {
  type ValueType = Vec2<f32>;
}

pub struct JointIndexChannel<const I: usize>;
impl<const I: usize> SemanticVertexShaderValue for JointIndexChannel<I> {
  // todo support u8 u16, currently the loader will expand the data to u32
  type ValueType = Vec4<u32>;
}

pub struct WeightChannel<const I: usize>;
impl<const I: usize> SemanticVertexShaderValue for WeightChannel<I> {
  type ValueType = Vec4<f32>;
}

// todo, use mat3 for none translation affine mat

only_vertex!(GeometryColor, Vec3<f32>);
only_vertex!(GeometryColorWithAlpha, Vec4<f32>);

both!(WorldNoneTranslationMatrix, Mat4<f32>);
both!(WorldPositionHP, HighPrecisionTranslation);

both!(WorldNormalMatrix, Mat3<f32>);
only_vertex!(VertexRenderPosition, Vec3<f32>);
only_vertex!(VertexRenderNormal, Vec3<f32>);

pub trait SemanticShaderValueExt {
  fn will_normal_computed_by_dxdy(&mut self) -> bool;
  /// gltf spec:
  ///
  /// When normals are not specified, client implementations MUST calculate flat normals and
  /// the provided tangents (if present) MUST be ignored.
  fn get_or_compute_fragment_normal(&mut self) -> Node<Vec3<f32>>;

  /// The user may not want shader variant over if the geometry has uv, so if the geometry
  /// does not have uv, we will just use (0., 0.) as default
  fn get_or_compute_fragment_uv(&mut self) -> Node<Vec2<f32>>;
}

impl SemanticShaderValueExt for ShaderFragmentBuilderView<'_> {
  fn will_normal_computed_by_dxdy(&mut self) -> bool {
    self.try_query::<FragmentRenderNormal>().is_none()
      && !self.has_vertex_value::<VertexRenderNormal>()
  }

  fn get_or_compute_fragment_normal(&mut self) -> Node<Vec3<f32>> {
    // check first and avoid unnecessary renormalize
    if let Some(normal) = self.try_query::<FragmentRenderNormal>() {
      normal
    } else if self.has_vertex_value::<VertexRenderNormal>() {
      let normal = self.query_or_interpolate_by::<FragmentRenderNormal, VertexRenderNormal>();
      let normal = normal.normalize(); // renormalize
      self.register::<FragmentRenderNormal>(normal);
      normal
    } else {
      let position = self.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();
      let normal = compute_normal_by_dxdy(position);
      self.register::<FragmentRenderNormal>(normal);
      normal
    }
  }

  fn get_or_compute_fragment_uv(&mut self) -> Node<Vec2<f32>> {
    if let Some(normal) = self.try_query::<FragmentUv>() {
      normal
    } else if self.has_vertex_value::<GeometryUV>() {
      let normal = self.query_or_interpolate_by::<FragmentUv, GeometryUV>();
      self.register::<FragmentUv>(normal);
      normal
    } else {
      let uv = val(Vec2::zero());
      self.register::<FragmentUv>(uv);
      uv
    }
  }
}

pub fn auto_reverse_normal_by_face_order(builder: &mut ShaderFragmentBuilderView) {
  if !builder.will_normal_computed_by_dxdy() {
    let normal = builder.get_or_compute_fragment_normal().make_local_var();
    if_by(builder.query::<FragmentFrontFacing>().not(), || {
      normal.store(-normal.load());
    });

    let normal = normal.load();
    builder.register::<FragmentRenderNormal>(normal);
  } else {
    builder.get_or_compute_fragment_normal();
  }
}

pub fn compute_normal_by_dxdy(position: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  // note, webgpu canvas is left handed
  position.dpdy().cross(position.dpdx()).normalize()
}

both!(CameraProjectionMatrix, Mat4<f32>);
both!(CameraProjectionInverseMatrix, Mat4<f32>);
both!(CameraWorldNoneTranslationMatrix, Mat4<f32>);
both!(CameraWorldPositionHP, HighPrecisionTranslation);

both!(CameraViewNoneTranslationProjectionMatrix, Mat4<f32>);
both!(CameraViewNoneTranslationProjectionInverseMatrix, Mat4<f32>);

both!(FragmentUv, Vec2<f32>);
both!(FragmentRenderPosition, Vec3<f32>);
both!(FragmentRenderNormal, Vec3<f32>);
both!(FragmentColor, Vec3<f32>);

both!(ViewportRenderBufferSize, Vec2<f32>);
both!(TexelSize, Vec2<f32>);

both!(ColorChannel, Vec3<f32>);

both!(EmissiveChannel, Vec3<f32>);
only_fragment!(HDRLightResult, Vec3<f32>);
only_fragment!(LDRLightResult, Vec3<f32>);
only_fragment!(ShouldUsePreSetLDRResult, bool);
