use std::marker::PhantomData;

use crate::*;

pub struct TraceBase<T>(PhantomData<T>);

impl<T> Default for TraceBase<T> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<T: Default + Copy + 'static> DeviceFutureProvider<T> for TraceBase<T> {
  fn build_device_future(&self) -> DynDeviceFuture<T> {
    BaseDeviceFuture::<T>::default().into_dyn()
  }
}
impl<T, Cx> NativeRayTracingShaderBuilder<Cx, T> for TraceBase<T>
where
  T: Default,
  Cx: NativeRayTracingShaderCtx,
{
  fn build(&self, _: &mut Cx) -> T {
    T::default()
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}

pub struct TraceNextRay<F, T> {
  upstream: T,
  next_trace_logic: F,
}

pub const TRACING_TASK_INDEX: usize = 0;
pub struct TracingTaskMarker;

pub trait TracingTaskSpawner {
  fn spawn_new_tracing_task(
    &mut self,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
  ) -> TaskFutureInvocationRightValue;
}

impl<F, T, O, P> DeviceFutureProvider<(O, Node<P>)> for TraceNextRay<F, T>
where
  T: DeviceFutureProvider<O>,
  F: FnOnce(O) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
  P: ShaderSizedValueNodeType + Default + Copy,
  O: ShaderAbstractRightValue,
{
  fn build_device_future(&self) -> DynDeviceFuture<(O, Node<P>)> {
    let next_trace_logic = self.next_trace_logic;
    self
      .upstream
      .build_device_future()
      .then(
        move |o, cx| {
          let (should_trace, trace, payload) = next_trace_logic(o);
          cx.registry
            .get_mut(&TypeId::of::<TracingTaskMarker>())
            .unwrap()
            .downcast_mut::<Box<dyn TracingTaskSpawner>>()
            .unwrap()
            .spawn_new_tracing_task(should_trace, trace, payload.handle(), P::sized_ty())
        },
        TaskFuture::<P>::new(TRACING_TASK_INDEX),
      )
      .into_dyn()
  }
}

impl<F, T, Cx, O, P> NativeRayTracingShaderBuilder<Cx, O> for TraceNextRay<F, T>
where
  T: NativeRayTracingShaderBuilder<Cx, O>,
  Cx: NativeRayTracingShaderCtx,
  F: FnOnce(&O) -> (Node<bool>, ShaderRayTraceCall, P) + Copy,
  P: 'static,
{
  fn build(&self, ctx: &mut Cx) -> O {
    let o = self.upstream.build(ctx);

    let (r, c, p) = (self.next_trace_logic)(&o);
    if_by(r, || {
      ctx.native_trace_ray(c, Box::new(p));
    });

    o
  }
  fn bind(&self, builder: &mut BindingBuilder) {
    self.upstream.bind(builder);
  }
}
