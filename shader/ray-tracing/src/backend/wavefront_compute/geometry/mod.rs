mod naive;
pub use naive::*;

use crate::*;

pub trait GPUAccelerationStructureSystemCompImplInstance: DynClone {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable>;
  fn bind_pass(&self, builder: &mut BindingBuilder);

  fn create_tlas_instance(&self) -> Box<dyn GPUAccelerationStructureSystemTlasCompImplInstance>;
}
clone_trait_object!(GPUAccelerationStructureSystemCompImplInstance);

pub trait GPUAccelerationStructureSystemTlasCompImplInstance: DynClone {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemTlasCompImplInvocation>;
  fn bind_pass(&self, builder: &mut BindingBuilder);
}
clone_trait_object!(GPUAccelerationStructureSystemTlasCompImplInstance);

pub trait GPUAccelerationStructureSystemCompImplInvocationTraversable {
  /// return optional closest hit
  fn traverse(
    &self,
    trace_payload: ENode<ShaderRayTraceCallStoragePayload>,
    user_defined_payloads: StorageNode<[u32]>,
    intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> ShaderOption<RayClosestHitCtx>;
}

pub trait GPUAccelerationStructureSystemTlasCompImplInvocation {
  fn index_tlas(
    &self,
    idx: Node<u32>,
  ) -> ReadOnlyStorageNode<TopLevelAccelerationStructureSourceDeviceInstance>;
}

#[derive(Clone, Copy)]
pub struct ShaderOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub(crate) struct Ray {
  pub origin: Vec3<f32>,
  pub flags: u32,
  pub direction: Vec3<f32>,
  pub mask: u32,
  // pub range: Vec2<f32>,
}

pub(crate) fn intersect_ray_aabb_cpu(
  ray_origin: Vec3<f32>,
  ray_direction: Vec3<f32>,
  ray_range: Vec2<f32>,
  box_min: Vec3<f32>,
  box_max: Vec3<f32>,
) -> bool {
  let t_min = ray_range.x();
  let t_max = ray_range.y();

  let inv_d = vec3(1., 1., 1.) / ray_direction;
  let t0 = (box_min - ray_origin) * inv_d;
  let t1 = (box_max - ray_origin) * inv_d;

  let t_near = t0.min(t1);
  let t_far = t0.max(t1);
  let t_near_max = t_near.max_channel();
  let t_far_min = t_far.min_channel();

  t_near_max <= t_far_min && t_min < t_far_min && t_near_max < t_max
}
pub(crate) fn intersect_ray_aabb_gpu(
  ray: Node<Ray>,
  box_min: Node<Vec3<f32>>,
  box_max: Node<Vec3<f32>>,
  near: Node<f32>,
  far: Node<f32>,
) -> Node<bool> {
  get_shader_fn::<bool>(shader_fn_name(intersect_ray_aabb_gpu))
    .or_define(|cx| {
      let ray = cx.push_fn_parameter_by(ray).expand();
      let box_min = cx.push_fn_parameter_by(box_min);
      let box_max = cx.push_fn_parameter_by(box_max);
      let t_min = cx.push_fn_parameter_by(near);
      let t_max = cx.push_fn_parameter_by(far);

      let inv_d = val(vec3(1., 1., 1.)) / ray.direction;
      let t0 = (box_min - ray.origin) * inv_d;
      let t1 = (box_max - ray.origin) * inv_d;

      let t_near = t0.min(t1);
      let t_far = t0.max(t1);
      let t_near_max = t_near.max_channel();
      let t_far_min = t_far.min_channel();

      let intersect = t_near_max
        .less_equal_than(t_far_min)
        .and(t_min.less_than(t_far_min))
        .and(t_near_max.less_than(t_max));
      cx.do_return(intersect)
    })
    .prepare_parameters()
    .push(ray)
    .push(box_min)
    .push(box_max)
    .push(near)
    .push(far)
    .call()
}

/// returns (hit, distance, u, v), hit = front hit -> 1, back hit -> -1, miss -> 0
fn intersect_ray_triangle_cpu(
  origin: Vec3<f32>,
  direction: Vec3<f32>,
  range: Vec2<f32>,
  v0: Vec3<f32>,
  v1: Vec3<f32>,
  v2: Vec3<f32>,
  cull_enable: bool,
  cull_back: bool,
) -> Vec4<f32> {
  let e1 = v1 - v0;
  let e2 = v2 - v0;
  let normal = e1.cross(e2).normalize();
  let b = normal.dot(direction);

  let sign = b.signum();

  if cull_enable {
    let pass = cull_back != (b < 0.);
    if !pass {
      return vec4(0., 0., 0., 0.);
    }
  }

  // todo cull
  let w0 = origin - v0;
  let a = -normal.dot(w0);
  let t = a / b;
  if t < range.x || t > range.y {
    return vec4(0., 0., 0., 0.);
  }

  let p = origin + t * direction;
  let uu = e1.dot(e1);
  let uv = e1.dot(e2);
  let vv = e2.dot(e2);
  let w = p - v0;
  let wu = w.dot(e1);
  let wv = w.dot(e2);
  let inverse_d = 1. / (uv * uv - uu * vv);
  let u = (uv * wv - vv * wu) * inverse_d;
  #[allow(clippy::manual_range_contains)]
  if u < 0. || u > 1. {
    return vec4(0., 0., 0., 0.);
  }
  let v = (uv * wu - uu * wv) * inverse_d;
  if v < 0. || (u + v) > 1. {
    return vec4(0., 0., 0., 0.);
  }
  vec4(sign, t, u, v)
}
/// returns (hit, distance, u, v), hit = front hit -> 1, back hit -> -1, miss -> 0
fn intersect_ray_triangle_gpu(
  origin: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  near: Node<f32>,
  far: Node<f32>,
  v0: Node<Vec3<f32>>,
  v1: Node<Vec3<f32>>,
  v2: Node<Vec3<f32>>,
  cull_enable: Node<bool>,
  cull_back: Node<bool>,
) -> Node<Vec4<f32>> {
  get_shader_fn::<Vec4<f32>>(shader_fn_name(intersect_ray_triangle_gpu))
    .or_define(|cx| {
      let origin = cx.push_fn_parameter_by(origin);
      let direction = cx.push_fn_parameter_by(direction);
      let near = cx.push_fn_parameter_by(near);
      let far = cx.push_fn_parameter_by(far);
      let v0 = cx.push_fn_parameter_by(v0);
      let v1 = cx.push_fn_parameter_by(v1);
      let v2 = cx.push_fn_parameter_by(v2);
      let cull_enable = cx.push_fn_parameter_by(cull_enable);
      let cull_back = cx.push_fn_parameter_by(cull_back);

      let e1 = v1 - v0;
      let e2 = v2 - v0;
      let normal = e1.cross(e2).normalize();
      let b = normal.dot(direction);
      let sign = b.sign();

      if_by(cull_enable, || {
        let is_front_facing = b.greater_than(val(0.));
        let pass = cull_back.not_equals(is_front_facing); // cull facing not equal to triangle facing
        if_by(pass, || {
          cx.do_return(val(vec4(0., 0., 0., 0.)));
        });
      });

      let w0 = origin - v0;
      let a = -normal.dot(w0);
      let t = a / b;

      let out_of_range = t.less_than(near).or(t.greater_than(far));
      if_by(out_of_range, || {
        cx.do_return(val(vec4(0., 0., 0., 0.)));
      });

      let p = origin + t * direction;
      let uu = e1.dot(e1);
      let uv = e1.dot(e2);
      let vv = e2.dot(e2);
      let w = p - v0;
      let wu = w.dot(e1);
      let wv = w.dot(e2);
      let inverse_d = val(1.) / (uv * uv - uu * vv);
      let u = (uv * wv - vv * wu) * inverse_d;

      let out_of_range = u.less_than(val(0.)).or(u.greater_than(val(1.)));
      if_by(out_of_range, || {
        cx.do_return(val(vec4(0., 0., 0., 0.)));
      });

      let v = (uv * wu - uu * wv) * inverse_d;
      let out_of_range = v.less_than(val(0.)).or((u + v).greater_than(val(1.)));
      if_by(out_of_range, || {
        cx.do_return(val(vec4(0., 0., 0., 0.)));
      });

      cx.do_return(Node::<Vec4<f32>>::from((sign, t, u, v)));
    })
    .prepare_parameters()
    .push(origin)
    .push(direction)
    .push(near)
    .push(far)
    .push(v0)
    .push(v1)
    .push(v2)
    .push(cull_enable)
    .push(cull_back)
    .call()
}
