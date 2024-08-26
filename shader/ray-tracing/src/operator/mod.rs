use crate::*;

pub trait TraceOperatorExt<T>: TraceOperator<T> + Sized {
  fn map<F, T2>(self, map: F) -> impl TraceOperator<T2>
  where
    F: FnOnce(T) -> T2 + 'static + Copy,
    T2: Default + ShaderAbstractRightValue,
    T: 'static,
  {
    TraceOutputMap {
      upstream_o: PhantomData,
      upstream: self,
      map,
    }
  }

  fn then_trace<F, P>(self, then: F) -> impl TraceOperator<(T, Node<P>)>
  where
    F: FnOnce(&T) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
    T: ShaderAbstractRightValue,
    P: ShaderSizedValueNodeType + Default + Copy,
  {
    TraceNextRay {
      upstream: self,
      next_trace_logic: then,
    }
  }
}
impl<T, X: TraceOperator<T>> TraceOperatorExt<T> for X {}

pub struct TraceOutputMap<F, T, O> {
  upstream_o: PhantomData<O>,
  upstream: T,
  map: F,
}

impl<O, O2, F, T> DeviceFutureProvider<O2> for TraceOutputMap<F, T, O>
where
  T: DeviceFutureProvider<O>,
  F: FnOnce(O) -> O2 + 'static + Copy,
  O2: Default + ShaderAbstractRightValue,
  O: 'static,
{
  fn build_device_future(&self) -> DynDeviceFuture<O2> {
    self.upstream.build_device_future().map(self.map).into_dyn()
  }
}

impl<F, T, O, O2> NativeRayTracingShaderBuilder<O2> for TraceOutputMap<F, T, O>
where
  T: NativeRayTracingShaderBuilder<O>,
  F: FnOnce(O) -> O2 + 'static + Copy,
{
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O2 {
    let o = self.upstream.build(ctx);
    (self.map)(o)
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.upstream.bind(builder);
  }
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
  F: FnOnce(&O) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
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
          let (should_trace, trace, payload) = next_trace_logic(&o);
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

impl<F, T, O, P> NativeRayTracingShaderBuilder<(O, Node<P>)> for TraceNextRay<F, T>
where
  T: NativeRayTracingShaderBuilder<O>,
  F: FnOnce(&O) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy,
  P: 'static,
{
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> (O, Node<P>) {
    let o = self.upstream.build(ctx);

    let (r, c, p) = (self.next_trace_logic)(&o);
    if_by(r, || {
      ctx.native_trace_ray(c, Box::new(p));
    });

    (o, p)
  }
  fn bind(&self, builder: &mut BindingBuilder) {
    self.upstream.bind(builder);
  }
}
