use crate::*;

pub struct TraceBase<T>(PhantomData<T>);

impl<T> Default for TraceBase<T> {
  fn default() -> Self {
    Self(Default::default())
  }
}

impl<T: Default + Copy + 'static> ShaderFutureProvider<T> for TraceBase<T> {
  fn build_device_future(&self, _: &mut AnyMap) -> DynShaderFuture<T> {
    BaseShaderFuture::<T>::default().into_dyn()
  }
}
impl<T> NativeRayTracingShaderBuilder<T> for TraceBase<T>
where
  T: Default,
{
  fn build(&self, _: &mut dyn NativeRayTracingShaderCtx) -> T {
    T::default()
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}

pub trait TraceOperatorExt<T>: TraceOperator<T> + Sized {
  fn map<F, T2>(self, map: F) -> impl TraceOperator<T2>
  where
    F: FnOnce(T, &mut TracingCtx) -> T2 + 'static + Copy,
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
    F: FnOnce(&T, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
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

impl<O, O2, F, T> ShaderFutureProvider<O2> for TraceOutputMap<F, T, O>
where
  T: ShaderFutureProvider<O>,
  F: FnOnce(O, &mut TracingCtx) -> O2 + 'static + Copy,
  O2: Default + ShaderAbstractRightValue,
  O: 'static,
{
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<O2> {
    let map = self.map;
    self
      .upstream
      .build_device_future(ctx)
      .map(move |o, cx| {
        let ctx = cx.invocation_registry.get_mut::<TracingCtx>().unwrap();
        map(o, ctx)
      })
      .into_dyn()
  }
}

impl<F, T, O, O2> NativeRayTracingShaderBuilder<O2> for TraceOutputMap<F, T, O>
where
  T: NativeRayTracingShaderBuilder<O>,
  F: FnOnce(O, &mut TracingCtx) -> O2 + 'static + Copy,
{
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> O2 {
    let o = self.upstream.build(ctx);
    (self.map)(o, ctx.tracing_ctx())
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

pub trait TracingTaskInvocationSpawner: DynClone {
  fn spawn_new_tracing_task(
    &mut self,
    task_group: &TaskGroupDeviceInvocationInstanceMaybeSelf,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
    parent_ref: TaskParentRef,
  ) -> TaskFutureInvocationRightValue;
}
impl Clone for Box<dyn TracingTaskInvocationSpawner> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

impl<F, T, O, P> ShaderFutureProvider<(O, Node<P>)> for TraceNextRay<F, T>
where
  T: ShaderFutureProvider<O>,
  F: FnOnce(&O, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy + 'static,
  P: ShaderSizedValueNodeType + Default + Copy,
  O: ShaderAbstractRightValue,
{
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<(O, Node<P>)> {
    let next_trace_logic = self.next_trace_logic;
    self
      .upstream
      .build_device_future(ctx)
      .then(
        move |o, then_invocation, cx| {
          let ctx = cx.invocation_registry.get_mut::<TracingCtx>().unwrap();
          let (should_trace, trace, payload) = next_trace_logic(&o, ctx);

          let parent = cx.generate_self_as_parent();
          cx.invocation_registry
            .get_mut::<Box<dyn TracingTaskInvocationSpawner>>()
            .unwrap()
            .spawn_new_tracing_task(
              &then_invocation.spawner,
              should_trace,
              trace,
              payload.handle(),
              P::sized_ty(),
              parent,
            )
        },
        TaskFuture::<P>::new(TRACING_TASK_INDEX),
      )
      .into_dyn()
  }
}

impl<F, T, O, P> NativeRayTracingShaderBuilder<(O, Node<P>)> for TraceNextRay<F, T>
where
  T: NativeRayTracingShaderBuilder<O>,
  F: FnOnce(&O, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>) + Copy,
  P: 'static,
{
  fn build(&self, ctx: &mut dyn NativeRayTracingShaderCtx) -> (O, Node<P>) {
    let o = self.upstream.build(ctx);

    let (r, c, p) = (self.next_trace_logic)(&o, ctx.tracing_ctx());
    if_by(r, || {
      ctx.native_trace_ray(c, Box::new(p));
    });

    (o, p)
  }
  fn bind(&self, builder: &mut BindingBuilder) {
    self.upstream.bind(builder);
  }
}
