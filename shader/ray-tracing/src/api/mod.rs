mod operator;
pub use operator::*;
mod ctx;
pub use ctx::*;

use crate::*;

pub trait ShaderAbstractLoadStore<T> {
  fn abstract_load(&self) -> T;
  fn abstract_store(&self, payload: T);
}
pub type BoxedShaderLoadStore<T> = Box<dyn ShaderAbstractLoadStore<T>>;

impl<T> ShaderAbstractLoadStore<Node<T>> for LocalVarNode<T> {
  fn abstract_load(&self) -> Node<T> {
    self.load()
  }
  fn abstract_store(&self, payload: Node<T>) {
    self.store(payload)
  }
}

#[derive(Clone, Copy)]
pub struct DevicePoll<T> {
  pub is_ready: Node<bool>,
  pub payload: T,
}

#[derive(Clone, Copy)]
pub struct DeviceOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}

// impl<T> From<(Node<bool>, T)> for DeviceOption<T> {
//   fn from((is_some, payload): (Node<bool>, T)) -> Self {
//     Self { is_some, payload }
//   }
// }

// impl<T: Copy> DeviceOption<T> {
//   pub fn some(payload: T) -> Self {
//     Self {
//       is_some: val(true),
//       payload,
//     }
//   }

//   pub fn map<U: ShaderSizedValueNodeType>(
//     self,
//     f: impl FnOnce(T) -> Node<U> + Copy,
//   ) -> DeviceOption<Node<U>> {
//     let u = zeroed_val().make_local_var();
//     if_by(self.is_some, || u.store(f(self.payload)));
//     (self.is_some, u.load()).into()
//   }
//   pub fn map_none<U: ShaderSizedValueNodeType>(
//     self,
//     f: impl FnOnce(T) -> Node<U> + Copy,
//   ) -> DeviceOption<Node<U>> {
//     let u = zeroed_val().make_local_var();
//     if_by(self.is_some.not(), || u.store(f(self.payload)));
//     (self.is_some, u.load()).into()
//   }
// }

pub trait DeviceFuture {
  type State;
  type Output;
  type Ctx;
  fn create_or_reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State;
  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DevicePoll<Self::Output>;
}

pub trait DeviceStateProvider {
  // todo, support PrimitiveShaderValueNodeType
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>>;
}

pub trait DeviceTaskSystem {
  /// argument must be valid for given task id to consume
  fn spawn_task<T>(&mut self, task_type: usize, argument: Node<T>) -> Node<u32>;
  fn poll_task<T>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool>;
}

/// impl native rtx support, the main difference between the future based impl
/// is the direct support of recursion call in shader
pub trait NativeRayTracingShaderBuilder {
  type Ctx;
  fn build(&self, ctx: &mut Self::Ctx);
}
pub trait NativeRayTracingShaderCtx {
  fn native_trace_ray(&self, ray: ShaderRayTraceCall);
}

pub trait ShaderRayGenLogic:
  DeviceFuture<Ctx = RayGenShaderCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayGenShaderCtx>
{
}

pub trait BoxShaderRayGenLogic {}

pub trait ShaderRayClosestHitLogic:
  DeviceFuture<Ctx = RayClosestHitCtx, Output = ()>
  + NativeRayTracingShaderBuilder<Ctx = RayClosestHitCtx>
{
}
pub trait BoxShaderRayClosestHitLogic {}

pub struct GPURaytracingPipelineBuilder {
  pub max_recursion_depth: u32,
  pub geometry_provider: Box<dyn GPUAccelerationStructureProvider>,
  ray_gen_shader: Box<dyn BoxShaderRayGenLogic>,
  // // miss_shader
  // miss_hit_shaders: Vec<Box<dyn BoxShaderRayClosestHitLogic>>,
  // intersection_shaders
  closest_hit_shaders: Vec<Box<dyn BoxShaderRayClosestHitLogic>>,
  // any_hit_shaders
}

impl Default for GPURaytracingPipelineBuilder {
  fn default() -> Self {
    Self {
      max_recursion_depth: 8,
      geometry_provider: todo!(),
      ray_gen_shader: todo!(),
      closest_hit_shaders: Default::default(),
    }
  }
}
pub struct GPURaytracingPipeline {
  pub internal: Box<dyn GPURaytracingPipelineProvider>,
}

pub enum RayAnyHitBehavior {
  IgnoreThisIntersect,
  TerminateTraverse,
}

impl GPURaytracingPipelineBuilder {
  pub fn with_max_recursion_depth(mut self, max_recursion_depth: u32) -> Self {
    self.max_recursion_depth = max_recursion_depth;
    self
  }

  pub fn with_ray_gen(self, ray_logic: impl ShaderRayGenLogic) -> Self {
    self
  }
  pub fn with_ray_intersection(
    self,
    range: usize,
    builder: impl FnOnce(&mut RayIntersectCtx),
  ) -> Self {
    self
  }

  pub fn with_ray_closest_hit(
    self,
    range: usize,
    ray_logic: impl ShaderRayClosestHitLogic,
  ) -> Self {
    self
  }

  pub fn with_ray_any_hit(
    self,
    range: usize,
    builder: impl FnOnce(&mut RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> Self {
    self
  }

  pub fn with_ray_miss(self, builder: impl FnOnce(&mut RayMissCtx)) -> Self {
    self
  }
}

// #[test]
// fn t() {
//   ray_ctx_from_declared_payload_input()
//     .then_trace_ray(|state, ctx| {
//       //
//     })
//     .then(|state, ctx| {
//       //
//     })
//     .then_trace_ray(|state, ctx| {
//       //
//     })
//     .then(|state, ctx| {
//       //
//     })
// }
