use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct TraceTaskSelfPayload {
  pub sub_task_ty: u32,
  pub sub_task_id: u32,
  pub trace_call: ShaderRayTraceCallStoragePayload,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct ShaderRayTraceCallStoragePayload {
  pub payload_ref: u32,
  pub tlas_idx: u32,
  pub ray_flags: u32,
  pub cull_mask: u32,
  pub sbt_ray_config_offset: u32,
  pub sbt_ray_config_stride: u32,
  pub miss_index: u32,
  pub ray_origin: Vec3<f32>,
  pub ray_direction: Vec3<f32>,
  pub range: Vec2<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct RayClosestHitCtxPayload {
  pub hit_ctx: ShaderRayTraceCallStoragePayload,
}

#[repr(C)]
#[std430_layout]
#[derive(ShaderStruct, Clone, Copy)]
pub struct RayMissHitCtxPayload {
  pub hit_ctx: ShaderRayTraceCallStoragePayload,
}

pub struct WaveFrontTracingBaseProvider;

impl TraceFutureBaseProvider for WaveFrontTracingBaseProvider {
  fn missing_shader_base<P: ShaderSizedValueNodeType>() -> impl TraceOperator<()> {
    CtxProviderTracer {
      is_missing_shader: true,
      payload_ty: P::sized_ty(),
    }
  }

  fn closest_shader_base<P: ShaderSizedValueNodeType>() -> impl TraceOperator<()> {
    CtxProviderTracer {
      is_missing_shader: false,
      payload_ty: P::sized_ty(),
    }
  }
}

struct CtxProviderTracer {
  is_missing_shader: bool,
  payload_ty: ShaderSizedValueType,
}

impl DeviceFutureProvider<()> for CtxProviderTracer {
  fn build_device_future(&self) -> DynDeviceFuture<()> {
    CtxProviderFuture {
      is_missing_shader: self.is_missing_shader,
      payload_ty: self.payload_ty.clone(),
    }
    .into_dyn()
  }
}
impl<T> NativeRayTracingShaderBuilder<T> for CtxProviderTracer
where
  T: Default,
{
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> T {
    T::default()
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}

pub struct CtxProviderFuture {
  is_missing_shader: bool,
  payload_ty: ShaderSizedValueType,
}

impl DeviceFuture for CtxProviderFuture {
  type Output = ();

  type Invocation = CtxProviderFutureInvocation;

  fn required_poll_count(&self) -> usize {
    1
  }

  fn build_poll(&self, _: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    CtxProviderFutureInvocation {
      is_missing_shader: self.is_missing_shader,
      payload_ty: self.payload_ty.clone(),
    }
  }

  fn bind_input(&self, _: &mut DeviceTaskSystemBindCtx) {}

  fn reset(&mut self, _: &mut DeviceParallelComputeCtx, _: u32) {}
}

pub struct CtxProviderFutureInvocation {
  is_missing_shader: bool,
  payload_ty: ShaderSizedValueType,
}
impl DeviceFutureInvocation for CtxProviderFutureInvocation {
  type Output = ();
  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> DevicePoll<()> {
    let combined_payload = ctx.access_self_payload_untyped();
    let payload: StorageNode<AnyType> = unsafe { index_access_field(combined_payload.handle(), 1) };

    let missing = self.is_missing_shader.then(|| unsafe {
      let ray_payload = index_access_field(combined_payload.handle(), 0);
      Box::new(ray_payload) as Box<dyn MissingHitCtxProvider>
    });

    let closest = (!self.is_missing_shader).then(|| unsafe {
      let ray_payload: StorageNode<RayClosestHitCtxPayload> =
        index_access_field(combined_payload.handle(), 0);
      Box::new(ray_payload) as Box<dyn ClosestHitCtxProvider>
    });

    ctx.invocation_registry.register(TracingCtx {
      missing,
      closest,
      payload: Some((payload, self.payload_ty.clone())),
    });
    // (val(true), self.0)
    // .into()
    todo!()
  }
}

impl RayLaunchInfoProvider for StorageNode<RayClosestHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    todo!()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    todo!()
  }
}

impl WorldRayInfoProvider for StorageNode<RayClosestHitCtxPayload> {
  fn world_ray(&self) -> ShaderRay {
    todo!()
  }

  fn ray_range(&self) -> ShaderRayRange {
    todo!()
  }

  fn ray_flags(&self) -> Node<u32> {
    todo!()
  }
}

impl ClosestHitCtxProvider for StorageNode<RayClosestHitCtxPayload> {
  fn primitive_id(&self) -> Node<u32> {
    todo!()
  }

  fn instance_id(&self) -> Node<u32> {
    todo!()
  }

  fn instance_custom_id(&self) -> Node<u32> {
    todo!()
  }

  fn geometry_id(&self) -> Node<u32> {
    todo!()
  }

  fn object_to_world(&self) -> Node<Mat4<f32>> {
    todo!()
  }

  fn world_to_object(&self) -> Node<Mat4<f32>> {
    todo!()
  }

  fn object_space_ray(&self) -> ShaderRay {
    todo!()
  }

  fn hit_kind(&self) -> Node<u32> {
    todo!()
  }

  fn hit_distance(&self) -> Node<f32> {
    todo!()
  }
}

impl RayLaunchInfoProvider for StorageNode<RayMissHitCtxPayload> {
  fn launch_id(&self) -> Node<Vec3<u32>> {
    todo!()
  }

  fn launch_size(&self) -> Node<Vec3<u32>> {
    todo!()
  }
}
impl WorldRayInfoProvider for StorageNode<RayMissHitCtxPayload> {
  fn world_ray(&self) -> ShaderRay {
    todo!()
  }

  fn ray_range(&self) -> ShaderRayRange {
    todo!()
  }

  fn ray_flags(&self) -> Node<u32> {
    todo!()
  }
}
impl MissingHitCtxProvider for StorageNode<RayMissHitCtxPayload> {}