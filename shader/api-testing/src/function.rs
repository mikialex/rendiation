use std::fmt::Debug;

use rendiation_shader_api::*;

use crate::harness::*;

#[shader_struct]
#[derive(Clone, Copy)]
pub struct FnRay {
  pub origin: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub enabled: Bool,
}

#[repr(C)]
#[shader_struct(std430)]
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct FnStd430Item {
  pub position: Vec3<f32>,
  pub weight: f32,
  pub flags: Vec2<u32>,
  pub id: u32,
}

/// the std140 struct is aligned to 16 bytes, so the shader struct has explicit padding members
/// after the field
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy, Default)]
pub struct FnStd140Inner {
  pub value: f32,
}

/// the nested struct field is aligned to 16 bytes in std140, so the explicit padding members are
/// inserted before it, and the field indices after it are remapped
#[repr(C)]
#[shader_struct(std140)]
#[derive(Clone, Copy, Default)]
pub struct FnStd140Outer {
  pub head: f32,
  pub inner: FnStd140Inner,
  pub tail: f32,
  pub direction: Vec3<f32>,
}

#[shader_fn]
fn scale_offset(v: Node<Vec3<f32>>, scale: Node<f32>, offset: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  v * scale + offset
}

#[shader_fn]
fn constant_one() -> Node<f32> {
  val(1.)
}

#[shader_fn]
fn chain_d(x: Node<u32>) -> Node<u32> {
  x * val(3) + val(1)
}

// the expressions before and after the nested definition belong to the same function
#[shader_fn]
fn chain_c(x: Node<u32>) -> Node<u32> {
  let before = x + val(2);
  let nested = chain_d_fn(before);
  nested + before
}

// chain_d is already defined by chain_c, it is only called here
#[shader_fn]
fn chain_b(x: Node<u32>) -> Node<u32> {
  chain_c_fn(x) * val(2) + chain_d_fn(x)
}

// the nested definitions begin inside the branches
#[shader_fn]
fn chain_a(x: Node<u32>) -> Node<u32> {
  let result = val(0_u32).make_local_var();
  if_by(x.greater_than(val(3)), || result.store(chain_b_fn(x)))
    .else_by(|| result.store(chain_c_fn(x) + val(1000)));
  result.load()
}

fn chain_d_cpu(x: u32) -> u32 {
  x * 3 + 1
}
fn chain_c_cpu(x: u32) -> u32 {
  chain_d_cpu(x + 2) + x + 2
}
fn chain_b_cpu(x: u32) -> u32 {
  chain_c_cpu(x) * 2 + chain_d_cpu(x)
}
fn chain_a_cpu(x: u32) -> u32 {
  if x > 3 {
    chain_b_cpu(x)
  } else {
    chain_c_cpu(x) + 1000
  }
}

/// The manual definition style.
fn manual_square(x: Node<i32>) -> Node<i32> {
  get_shader_fn::<i32>(shader_fn_name(manual_square))
    .or_define(|cx| {
      let x = cx.push_fn_parameter_by(x);
      cx.do_return(x * x);
    })
    .prepare_parameters()
    .push(x)
    .call()
}

/// The manual definition style with the parameters pushed by type, the body defines another
/// function.
fn manual_sum_of_squares(a: Node<i32>, b: Node<i32>) -> Node<i32> {
  get_shader_fn::<i32>(shader_fn_name(manual_sum_of_squares))
    .or_define(|cx| {
      let a = cx.push_fn_parameter::<i32>();
      let b = cx.push_fn_parameter::<i32>();
      cx.do_return(manual_square(a) + manual_square(b));
    })
    .prepare_parameters()
    .push(a)
    .push(b)
    .call()
}

/// Every kind of return point: the early return in the nested branch, in the switch case and in
/// the loop, and the final return.
fn classify(x: Node<u32>) -> Node<u32> {
  get_shader_fn::<u32>(shader_fn_name(classify))
    .or_define(|cx| {
      let x = cx.push_fn_parameter_by(x);
      if_by(x.less_than(val(10)), || {
        if_by(x.equals(0), || cx.do_return(val(100)));
        if_by((x % val(2)).equals(1), || cx.do_return(x + val(200))).else_by(|| {});
      });
      switch_by(x)
        .case(10, || cx.do_return(val(300)))
        .case(11, || {})
        .end_with_default(|| {});
      let sum = val(0_u32).make_local_var();
      let i = val(0_u32).make_local_var();
      loop_by(|lp| {
        let iv = i.load();
        if_by(iv.greater_equal_than(x), || lp.do_break());
        i.store(iv + val(1));
        if_by((iv % val(3)).equals(0), || lp.do_continue());
        sum.store(sum.load() + iv);
        if_by(sum.load().greater_than(val(50)), || {
          cx.do_return(iv + val(1000))
        });
      });
      cx.do_return(sum.load());
    })
    .prepare_parameters()
    .push(x)
    .call()
}

fn classify_cpu(x: u32) -> u32 {
  if x < 10 {
    if x == 0 {
      return 100;
    }
    if x % 2 == 1 {
      return x + 200;
    }
  }
  if x == 10 {
    return 300;
  }
  let mut sum = 0;
  for i in 0..x {
    if i % 3 == 0 {
      continue;
    }
    sum += i;
    if sum > 50 {
      return i + 1000;
    }
  }
  sum
}

// the early return of the macro style function is written by return_value, the tail value is the
// last return
#[shader_fn]
fn early_return_macro(x: Node<f32>) -> Node<f32> {
  if_by(x.less_than(val(2.)), || return_value(Some(val(-1.))));
  let local = x.make_local_var();
  loop_by(|cx| {
    let v = local.load();
    if_by(v.less_than(val(10.)), || cx.do_break());
    local.store(v - val(7.));
  });
  local.load() * val(2.)
}

fn early_return_macro_cpu(x: f32) -> f32 {
  if x < 2. {
    return -1.;
  }
  let mut v = x;
  while v >= 10. {
    v -= 7.;
  }
  v * 2.
}

#[shader_fn]
fn in_range(x: Node<u32>, low: Node<u32>, high: Node<u32>) -> Node<bool> {
  x.greater_equal_than(low).and(x.less_than(high))
}

#[shader_fn]
fn pick(condition: Node<bool>, accept: Node<u32>, reject: Node<u32>) -> Node<u32> {
  condition.select(accept, reject)
}

// the nested definitions begin inside the loop and the switch cases of the outer function
#[shader_fn]
fn loop_caller(x: Node<u32>) -> Node<u32> {
  let sum = val(0_u32).make_local_var();
  x.into_shader_iter().for_each(|i, cx| {
    switch_by(i % val(3))
      .case(0, || sum.store(sum.load() + loop_callee_a_fn(i)))
      .case(1, || cx.do_continue())
      .end_with_default(|| sum.store(sum.load() + loop_callee_b_fn(i)));
    if_by(sum.load().greater_than(val(100)), || cx.do_break());
  });
  sum.load()
}

#[shader_fn]
fn loop_callee_a(x: Node<u32>) -> Node<u32> {
  x * val(5) + val(1)
}

#[shader_fn]
fn loop_callee_b(x: Node<u32>) -> Node<u32> {
  x.greater_than(val(4))
    .select_branched(|| x * val(2), || x + val(7))
}

fn loop_caller_cpu(x: u32) -> u32 {
  let mut sum = 0;
  for i in 0..x {
    match i % 3 {
      0 => sum += i * 5 + 1,
      1 => continue,
      _ => sum += if i > 4 { i * 2 } else { i + 7 },
    }
    if sum > 100 {
      break;
    }
  }
  sum
}

#[shader_fn]
fn make_ray(x: Node<f32>, enabled: Node<bool>) -> Node<FnRay> {
  ENode::<FnRay> {
    origin: vec3_node((val(1.), x, val(0.))),
    direction: val(Vec3::new(0., -2., 0.)),
    enabled: enabled.into_big_bool(),
  }
  .construct()
}

#[shader_fn]
fn ray_plane_distance(ray: Node<FnRay>) -> Node<f32> {
  let ray = ray.expand();
  if_by(ray.enabled.into_bool().not(), || {
    return_value(Some(val(-1.)))
  });
  -ray.origin.y() / ray.direction.y() + ray.origin.x()
}

#[shader_fn]
fn process_item(item: Node<FnStd430Item>, scale: Node<f32>) -> Node<FnStd430Item> {
  let item = item.expand();
  let flags = item.flags.yx();
  ENode::<FnStd430Item> {
    position: item.position * scale,
    weight: item.weight + scale,
    flags,
    id: item.id + flags.x(),
  }
  .construct()
}

#[shader_fn]
fn std140_shuffle(s: Node<FnStd140Outer>, extra: Node<FnStd140Inner>) -> Node<FnStd140Outer> {
  let s = s.expand();
  let inner = s.inner.expand();
  let extra = extra.expand();
  ENode::<FnStd140Outer> {
    head: inner.value + extra.value,
    inner: ENode::<FnStd140Inner> { value: s.tail }.construct(),
    tail: s.head,
    direction: s.direction * val(2.),
  }
  .construct()
}

// the fields of the padded struct are accessed through the pointer of the local variable
#[shader_fn]
fn std140_through_pointer(s: Node<FnStd140Outer>) -> Node<FnStd140Outer> {
  let local = s.make_local_var();
  let tail = local.tail().load();
  let value = local.inner().value();
  value.store(value.load() * val(10.) + tail);
  local.tail().store(FnStd140Outer::head(s));
  local.direction().store(local.direction().load().zyx());
  local.load()
}

#[shader_fn]
fn make_matrix(f: Node<f32>) -> Node<Mat4<f32>> {
  mat4_node((
    vec4_node((f, val(0.), val(0.), val(0.))),
    val(Vec4::new(0., 1., 0., 0.)),
    val(Vec4::new(0., 0., 1., 0.)),
    vec4_node((val(1.), f, val(2.), val(1.))),
  ))
}

#[shader_fn]
fn matrix_combine(
  m3: Node<Mat3<f32>>,
  m4: Node<Mat4<f32>>,
  m2x3: Node<Mat2x3<f32>>,
  v: Node<Vec3<f32>>,
) -> Node<Vec4<f32>> {
  let a = m3 * v;
  let b = m4 * vec4_node((v, val(1.)));
  let c = m2x3.transpose() * v;
  vec4_node((
    a.x() + a.y() + a.z(),
    b.x() + b.w(),
    c.x() - c.y(),
    m3.determinant(),
  ))
}

#[shader_fn]
fn array_reverse(array: Node<[u32; 4]>) -> Node<[u32; 4]> {
  let source = array.make_local_var();
  let result = make_local_var::<[u32; 4]>();
  for i in 0..4 {
    result.index(i).store(source.index(3 - i).load());
  }
  result.load()
}

/// The array parameter is indexed by the runtime value through a local variable.
fn array_tail_sum(array: Node<[u32; 4]>, start: Node<u32>) -> Node<u32> {
  get_shader_fn::<u32>(shader_fn_name(array_tail_sum))
    .or_define(|cx| {
      let array = cx.push_fn_parameter_by(array).make_local_var();
      let start = cx.push_fn_parameter_by(start);
      let sum = (start..val(4))
        .into_shader_iter()
        .map(|i| array.index(i).load())
        .sum();
      cx.do_return(sum);
    })
    .prepare_parameters()
    .push(array)
    .push(start)
    .call()
}

#[shader_fn]
fn constants_and_locals(x: Node<f32>) -> Node<Vec4<f32>> {
  let base = val(Vec3::new(1., 2., 3.));
  // the constant is composed with the runtime value
  let composed = vec4_node((base, x));
  let named = global_const_val(Vec4::new(0.5, 0.25, 2., 4.));
  let acc = zeroed_val::<Vec4<f32>>().make_local_var();
  val(3_u32).into_shader_iter().for_each(|i, _| {
    acc.store(acc.load() + composed * i.into_f32());
  });
  acc.load() + named
}

#[shader_fn]
fn texture_lookup(
  tex: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
) -> Node<Vec4<f32>> {
  tex.sample_zero_level(sampler, uv)
}

#[shader_fn]
fn sample_with_derivative(
  tex: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
) -> Node<Vec4<f32>> {
  let color = tex.sample(sampler, uv);
  color + vec4_node((uv.dpdx(), uv.dpdy()))
}

/// Pass the value through a function parameter, a local variable and the return value.
fn pass_through<T>(name: &str, v: Node<T>) -> Node<T>
where
  T: ShaderSizedValueNodeType + SizedShaderAbstractPtrAccess,
{
  get_shader_fn::<T>(format!("pass_through_{name}"))
    .or_define(|cx| {
      let v = cx.push_fn_parameter_by(v);
      cx.do_return(v.make_local_var().load());
    })
    .prepare_parameters()
    .push(v)
    .call()
}

fn find_fn<'a>(module: &'a naga::Module, name: &str) -> &'a naga::Function {
  module
    .functions
    .iter()
    .map(|(_, f)| f)
    .find(|f| f.name.as_deref().is_some_and(|n| n.ends_with(name)))
    .unwrap_or_else(|| panic!("function {name} not found"))
}

fn has_named_expression(f: &naga::Function, name: &str) -> bool {
  f.named_expressions.values().any(|n| n == name)
}

/// the macro style function, with the zero parameter function, called from the entry
#[test]
fn fn_macro_style() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;
    let v = f.splat::<Vec3<f32>>();
    keep(scale_offset_fn(v, f, v));
    keep(scale_offset_fn(
      v,
      constant_one_fn(),
      val(Vec3::new(1., 2., 3.)),
    ));
  });
}

/// the manual definition style, the parameters pushed by the node or by the type
#[test]
fn fn_manual_style() {
  check_compute(|builder| {
    let i = runtime_values(builder).i;
    keep(manual_square(i));
    keep(manual_sum_of_squares(i, i + val(1)));
  });
}

/// the nested definitions several levels deep, the definition begins in the branch of the outer
/// function, and the nested function is called again after it is defined
#[test]
fn fn_nested_definition() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    keep(chain_a_fn(u));
    keep(loop_caller_fn(u));
  });
}

/// the nested definitions begin inside the loop, the switch case and the branch of the entry
#[test]
fn fn_defined_in_entry_control_flow() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    loop_by(|cx| {
      keep(chain_d_fn(u));
      cx.do_break();
    });
    switch_by(u)
      .case(0, || keep(chain_b_fn(u)))
      .end_with_default(|| keep(chain_c_fn(u)));
    if_by(u.equals(1), || keep(chain_a_fn(u)));
    keep(u.into_shader_iter().map(loop_callee_a_fn).sum());
  });
}

/// each function is defined only once, no matter how many times and where it is called, the typed
/// function handle can be called multiple times
#[test]
fn fn_defined_once() {
  let module = build_compute(|builder| {
    let u = runtime_values(builder).u;
    keep(chain_d_fn(u));
    keep(chain_a_fn(u));
    keep(chain_a_fn(u + val(1)));
    keep(chain_c_fn(u) + chain_b_fn(u));

    let define = || {
      get_shader_fn::<u32>("fn_defined_once".to_string()).or_define(|cx| {
        let x = cx.push_fn_parameter_by(u);
        cx.do_return(chain_d_fn(x) + chain_d_fn(x + val(1)));
      })
    };
    let typed = define();
    keep(typed.clone().prepare_parameters().push(u).call());
    keep(typed.prepare_parameters().push(u + val(1)).call());
    keep(define().prepare_parameters().push(u).call());
  });
  validate(&module);
  assert_eq!(module.functions.len(), 5);
}

/// the call results are the arguments of other calls, and the else if condition is a call that
/// defines the function between the if and the else if
#[test]
fn fn_call_as_argument_and_condition() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    keep(pick_fn(
      in_range_fn(u, val(2), val(5)),
      chain_d_fn(u),
      chain_c_fn(u),
    ));
    let r = val(0_u32).make_local_var();
    if_by(u.equals(0), || r.store(val(1)))
      .else_if(in_range_fn(u, val(2), val(5)), || r.store(chain_b_fn(u)))
      .else_by(|| r.store(val(3)));
    keep(r.load());
  });
}

/// the control flow inside the function: branch, switch, loop with break and continue, and the
/// early returns from them
#[test]
fn fn_control_flow() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    keep(classify(u));
    keep(early_return_macro_fn(f));
    keep(loop_callee_b_fn(u));
  });
}

/// all kinds of the parameter and return types
#[test]
fn fn_parameter_and_return_types() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    keep(pass_through("f32", f));
    keep(pass_through("u32", u));
    keep(pass_through("i32", i));
    keep(pass_through("bool", u.equals(0)));
    keep(pass_through("vec2_f32", f.splat::<Vec2<f32>>()));
    keep(pass_through("vec3_u32", u.splat::<Vec3<u32>>()));
    keep(pass_through("vec4_i32", i.splat::<Vec4<i32>>()));
    keep(pass_through("vec2_bool", u.equals(0).splat::<Vec2<bool>>()));
    keep(pass_through("mat2", zeroed_val::<Mat2<f32>>() * f));
    keep(pass_through("mat3", zeroed_val::<Mat3<f32>>() * f));
    keep(pass_through("mat4", make_matrix_fn(f)));
    keep(pass_through("mat2x3", zeroed_val::<Mat2x3<f32>>()));
    keep(pass_through("mat3x2", zeroed_val::<Mat3x2<f32>>()));
    keep(pass_through("mat2x4", zeroed_val::<Mat2x4<f32>>()));
    keep(pass_through("mat3x4", zeroed_val::<Mat3x4<f32>>()));
    keep(pass_through("mat4x2", zeroed_val::<Mat4x2<f32>>()));
    keep(pass_through("mat4x3", zeroed_val::<Mat4x3<f32>>()));
    keep(pass_through("struct", make_ray_fn(f, u.equals(0))));
    keep(pass_through("std430", zeroed_val::<FnStd430Item>()));
    keep(pass_through("std140_inner", zeroed_val::<FnStd140Inner>()));
    keep(pass_through("std140_outer", zeroed_val::<FnStd140Outer>()));
    keep(pass_through("array", zeroed_val::<[u32; 4]>()));
    keep(pass_through("struct_array", zeroed_val::<[FnRay; 2]>()));
    keep(pass_through(
      "std140_array",
      zeroed_val::<[FnStd140Outer; 2]>(),
    ));
  });
}

/// the struct parameters and returns, the fields of the parameter are read and the returned
/// struct is constructed, including the host layout structs with explicit padding members
#[test]
fn fn_struct_parameter_and_return() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    keep(ray_plane_distance_fn(make_ray_fn(f, u.equals(0))));

    let item = ENode::<FnStd430Item> {
      position: f.splat(),
      weight: f,
      flags: u.splat(),
      id: u,
    };
    keep(process_item_fn(item.construct(), f).expand().id);

    let inner = ENode::<FnStd140Inner> { value: f }.construct();
    let outer = ENode::<FnStd140Outer> {
      head: f,
      inner,
      tail: f,
      direction: f.splat(),
    };
    let result = std140_shuffle_fn(outer.construct(), inner).expand();
    keep(result.inner.expand().value + result.tail);
    keep(std140_through_pointer_fn(outer.construct()));
  });
}

/// the many parameters with mixed types, like the ray triangle intersection function
#[test]
fn fn_many_parameters() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let v = f.splat::<Vec3<f32>>();
    let flag = u.equals(0);
    let r = get_shader_fn::<Vec4<f32>>("fn_many_parameters".to_string())
      .or_define(|cx| {
        let origin = cx.push_fn_parameter_by(v);
        let direction = cx.push_fn_parameter_by(v);
        let near = cx.push_fn_parameter_by(f);
        let far = cx.push_fn_parameter_by(f);
        let v0 = cx.push_fn_parameter_by(v);
        let v1 = cx.push_fn_parameter_by(v);
        let v2 = cx.push_fn_parameter_by(v);
        let cull_enable = cx.push_fn_parameter_by(flag);
        let cull_back = cx.push_fn_parameter_by(flag);
        if_by(cull_enable.and(cull_back), || {
          cx.do_return(val(Vec4::new(0., 0., 0., 0.)))
        });
        let e = (v1 - v0).cross(v2 - v0);
        cx.do_return(vec4_node((e.dot(direction), near, far, origin.x())));
      })
      .prepare_parameters()
      .push(v)
      .push(v)
      .push(f)
      .push(f)
      .push(v)
      .push(v)
      .push(v)
      .push(flag)
      .push(flag)
      .call();
    keep(r);
  });
}

/// the texture and sampler parameters, different textures are passed to the same function
#[test]
fn fn_texture_parameter() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;
    let tex_a: BindingNode<ShaderTexture2D> = fake_binding(0);
    let tex_b: BindingNode<ShaderTexture2D> = fake_binding(1);
    let sampler: BindingNode<ShaderSampler> = fake_binding(2);
    keep(texture_lookup_fn(tex_a, sampler, f.splat()));
    keep(texture_lookup_fn(tex_b, sampler, f.splat()));
  });
}

/// the barriers and the assertion inside the function called in the uniform control flow
#[test]
fn fn_barrier_and_assertion() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let r = get_shader_fn::<u32>("fn_barrier_and_assertion".to_string())
      .or_define(|cx| {
        let x = cx.push_fn_parameter_by(u);
        shader_assert(x.less_than(val(1024)));
        workgroup_barrier();
        storage_barrier();
        cx.do_return(x);
      })
      .prepare_parameters()
      .push(u)
      .call();
    keep(r);
  });
}

/// the constants, the global constant and the local variables in the function body
#[test]
fn fn_constants_and_locals() {
  check_compute(|builder| {
    keep(constants_and_locals_fn(runtime_values(builder).f));
    keep(constants_and_locals_fn(val(1.)));
  });
}

/// the debug labels on the parameter, the local variable and the expressions are attached to the
/// function they belong to, the nested definition between them does not affect the labels, the
/// macro style function labels the parameters and the let bindings automatically
#[test]
fn fn_debug_labels() {
  let module = build_compute(|builder| {
    let f = runtime_values(builder).f;
    let r = get_shader_fn::<f32>("fn_debug_labels".to_string())
      .or_define(|cx| {
        let param = cx.push_fn_parameter_by(f);
        param.mark_debug_label("param");
        let local = param.make_local_var();
        unsafe { local.raw().get_raw_ptr().into_node::<AnyType>() }.mark_debug_label("local");
        let nested = early_return_macro_fn(param);
        nested.mark_debug_label("nested_result");
        let doubled = nested * val(2.);
        doubled.mark_debug_label("doubled");
        local.store(local.load() + doubled);
        cx.do_return(local.load());
      })
      .prepare_parameters()
      .push(f)
      .call();
    keep(r);
  });
  validate(&module);

  let labeled = find_fn(&module, "fn_debug_labels");
  assert_eq!(labeled.arguments[0].name.as_deref(), Some("param"));
  assert!(
    labeled
      .local_variables
      .iter()
      .any(|(_, v)| v.name.as_deref() == Some("local"))
  );
  assert!(has_named_expression(labeled, "nested_result"));
  assert!(has_named_expression(labeled, "doubled"));

  let macro_style = find_fn(&module, "::early_return_macro");
  assert_eq!(macro_style.arguments[0].name.as_deref(), Some("x"));
  assert!(has_named_expression(macro_style, "v"));
  assert!(!has_named_expression(macro_style, "doubled"));

  let entry = &module.entry_points[0].function;
  assert!(!has_named_expression(entry, "doubled"));
  assert!(!has_named_expression(entry, "v"));
}

both!(FnVaryingValue, f32);

/// the same function is called in the vertex and the fragment stage, each stage module defines
/// its own copy
#[test]
fn fn_in_both_graphics_stages() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>();
      builder.set_vertex_out::<FnVaryingValue>(early_return_macro_fn(index.into_f32()));
      keep(chain_a_fn(index));
    });
    builder.fragment(|builder, _| {
      let v = builder.query::<FnVaryingValue>();
      keep(early_return_macro_fn(v) + chain_a_fn(v.into_u32()).into_f32());
    });
  });
}

/// the entry outputs are assembled when the entry function ends instead of when a function
/// definition ends: the outputs are written by the call results, and the functions are defined
/// before and after the outputs are written
#[test]
fn fn_with_entry_outputs() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>();
      let f = index.into_f32();
      let position = make_matrix_fn(f) * vec4_node((f.splat::<Vec3<f32>>(), val(1.)));
      builder.register::<ClipPosition>(position);
      builder.set_vertex_out::<FnVaryingValue>(early_return_macro_fn(f));
      keep(loop_caller_fn(index));
    });
    builder.fragment(|builder, _| {
      let v = builder.query::<FnVaryingValue>();
      let slot = builder.define_out_by(channel(TextureFormat::Rgba8Unorm));
      builder.store_fragment_out_vec4f(slot, constants_and_locals_fn(v));
      keep(chain_a_fn(v.into_u32()));
    });
  });
}

/// the fragment only operations inside the function called by the fragment stage: the implicit
/// level sampling with the texture parameter, the derivatives and the discard
#[test]
fn fn_fragment_only_operations() {
  check_graphics(|builder| {
    builder.vertex(|builder, _| {
      let index = builder.query::<VertexIndex>().into_f32();
      builder.set_vertex_out::<FnVaryingValue>(index);
    });
    builder.fragment(|builder, _| {
      let v = builder.query::<FnVaryingValue>();
      let tex: BindingNode<ShaderTexture2D> = fake_binding(0);
      let sampler: BindingNode<ShaderSampler> = fake_binding(1);
      keep(sample_with_derivative_fn(tex, sampler, v.splat()));
      let r = get_shader_fn::<f32>("fn_fragment_discard".to_string())
        .or_define(|cx| {
          let x = cx.push_fn_parameter_by(v);
          if_by(x.less_than(val(0.5)), || builder.discard());
          cx.do_return(x);
        })
        .prepare_parameters()
        .push(v)
        .call();
      keep(r);
    });
  });
}

/// WGSL does not allow recursion, the function can not call itself
#[test]
#[should_panic(expected = "recursive fn definition is not allowed")]
fn fn_recursive_definition() {
  build_compute(|builder| {
    keep(recursive(runtime_values(builder).u));
  });
}

/// A function that calls itself in its body.
fn recursive(x: Node<u32>) -> Node<u32> {
  get_shader_fn::<u32>(shader_fn_name(recursive))
    .or_define(|cx| {
      let x = cx.push_fn_parameter_by(x);
      cx.do_return(recursive(x - val(1)));
    })
    .prepare_parameters()
    .push(x)
    .call()
}

#[shader_fn]
fn recursive_macro(x: Node<u32>) -> Node<u32> {
  x.equals(0).select(val(0), recursive_macro_fn(x - val(1)))
}

/// the same case in the macro style
#[test]
#[should_panic(expected = "recursive fn definition is not allowed")]
fn fn_recursive_definition_macro() {
  build_compute(|builder| {
    keep(recursive_macro_fn(runtime_values(builder).u));
  });
}

#[shader_fn]
fn mutual_a(x: Node<u32>) -> Node<u32> {
  mutual_b_fn(x) + val(1)
}

#[shader_fn]
fn mutual_b(x: Node<u32>) -> Node<u32> {
  mutual_a_fn(x) * val(2)
}

/// the indirect recursion is rejected too
#[test]
#[should_panic(expected = "recursive fn definition is not allowed")]
fn fn_mutual_recursive_definition() {
  build_compute(|builder| {
    keep(mutual_a_fn(runtime_values(builder).u));
  });
}

/// the function name is unique in the module, the definition can not begin again
#[test]
#[should_panic(expected = "function redefinition")]
fn fn_redefinition() {
  build_compute(|builder| {
    keep(manual_square(runtime_values(builder).i));
    FunctionBuildCtx::<i32>::begin(shader_fn_name(manual_square));
  });
}

/// the loop outside of a function can not be continued inside the function
#[test]
#[should_panic(expected = "continue a loop outside of the loop")]
fn fn_continue_outer_loop() {
  build_compute(|_| {
    loop_by(|cx| {
      get_shader_fn::<f32>("fn_continue_outer_loop".to_string()).or_define(|builder| {
        cx.do_continue();
        builder.do_return(val(0.));
      });
      cx.do_break();
    });
  });
}

/// the iterator created outside of the function can not be iterated inside it
#[test]
#[should_panic(
  expected = "the shader iterator is created outside of the function and iterated inside it"
)]
fn fn_iter_created_outside() {
  build_compute(|_| {
    let iter = 4_u32.into_shader_iter();
    get_shader_fn::<u32>("fn_iter_created_outside".to_string()).or_define(|cx| {
      cx.do_return(iter.sum());
    });
  });
}

/// the function reads the storage buffer declared in the entry, WGSL functions can access the
/// module scope variables
#[test]
#[ignore = "bug: the global variable expression only exists in the entry function arena"]
fn fn_reads_storage_buffer() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let storage = fake_storage_buffer::<[u32]>(0);
    let r = get_shader_fn::<u32>("fn_reads_storage_buffer".to_string())
      .or_define(|cx| {
        let i = cx.push_fn_parameter_by(u);
        cx.do_return(storage.index(i).load());
      })
      .prepare_parameters()
      .push(u)
      .call();
    keep(r);
  });
}

/// the function writes the workgroup variable declared in the entry
#[test]
#[ignore = "bug: the global variable expression only exists in the entry function arena"]
fn fn_writes_workgroup_variable() {
  check_compute(|builder| {
    let u = runtime_values(builder).u;
    let shared = builder.define_workgroup_shared_var::<u32>();
    let r = get_shader_fn::<u32>("fn_writes_workgroup_variable".to_string())
      .or_define(|cx| {
        let i = cx.push_fn_parameter_by(u);
        shared.store(i);
        cx.do_return(i);
      })
      .prepare_parameters()
      .push(u)
      .call();
    keep(r + shared.load());
  });
}

/// Run the shader logic on the GPU for each input value, and compare with the cpu reference logic.
async fn check_fn<O>(shader: impl Fn(Node<u32>) -> Node<O> + 'static, cpu: impl Fn(u32) -> O)
where
  O: Std430 + ShaderSizedValueNodeType + PartialEq + Debug,
{
  let input: Vec<_> = (0..24).collect();
  let expect: Vec<_> = input.iter().map(|v| cpu(*v)).collect();
  assert_eq!(gpu_map(&input, shader).await, expect);
}

/// the nested definitions, the reused function and the macro and manual style
#[pollster::test]
async fn fn_gpu_nested_definition() {
  check_fn(
    |v| chain_a_fn(v) + chain_d_fn(v) * val(10000),
    |v| chain_a_cpu(v) + chain_d_cpu(v) * 10000,
  )
  .await;
  check_fn(
    |v| manual_sum_of_squares(v.into_i32() - val(10), val(3)) + manual_square(v.into_i32()),
    |v| {
      let v = v as i32;
      (v - 10) * (v - 10) + 9 + v * v
    },
  )
  .await;
}

/// the nested definitions begin inside the loop, the switch case and the branch of the entry,
/// and the function is called again in the other branches
#[pollster::test]
async fn fn_gpu_defined_in_entry_control_flow() {
  check_fn(
    |v| {
      let sum = val(0_u32).make_local_var();
      v.into_shader_iter()
        .for_each(|i, _| sum.store(sum.load() + chain_d_fn(i)));
      switch_by(v % val(3))
        .case(0, || sum.store(sum.load() + chain_b_fn(v)))
        .case(1, || sum.store(sum.load() + chain_c_fn(v) * val(2)))
        .end_with_default(|| sum.store(sum.load() + chain_b_fn(v) * val(3)));
      if_by(v.greater_than(val(5)), || {
        sum.store(sum.load() + chain_a_fn(v))
      });
      sum.load()
    },
    |v| {
      let mut sum: u32 = (0..v).map(chain_d_cpu).sum();
      sum += match v % 3 {
        0 => chain_b_cpu(v),
        1 => chain_c_cpu(v) * 2,
        _ => chain_b_cpu(v) * 3,
      };
      if v > 5 {
        sum += chain_a_cpu(v);
      }
      sum
    },
  )
  .await;
}

/// every return path of the function: the early return in the nested branch, in the switch case
/// and in the loop, and the final return
#[pollster::test]
async fn fn_gpu_return_paths() {
  check_fn(classify, classify_cpu).await;
  check_fn(
    |v| early_return_macro_fn(v.into_f32()),
    |v| early_return_macro_cpu(v as f32),
  )
  .await;
}

/// the loop with break and continue, and the nested definitions inside the loop and the switch
/// cases of the outer function
#[pollster::test]
async fn fn_gpu_loop_in_function() {
  check_fn(loop_caller_fn, loop_caller_cpu).await;
}

/// the bool parameter and return, the call results as the arguments, and the call in the else if
/// condition
#[pollster::test]
async fn fn_gpu_bool_and_call_arguments() {
  check_fn(
    |v| pick_fn(in_range_fn(v, val(3), val(9)), chain_d_fn(v), v + val(100)),
    |v| {
      if (3..9).contains(&v) {
        chain_d_cpu(v)
      } else {
        v + 100
      }
    },
  )
  .await;
  check_fn(
    |v| {
      let r = val(0_u32).make_local_var();
      if_by(v.equals(0), || r.store(val(1)))
        .else_if(in_range_fn(v, val(2), val(5)), || r.store(chain_d_fn(v)))
        .else_by(|| r.store(val(3)));
      r.load()
    },
    |v| match v {
      0 => 1,
      2..5 => chain_d_cpu(v),
      _ => 3,
    },
  )
  .await;
}

/// the struct with the bool field is constructed in one function and read in another one
#[pollster::test]
async fn fn_gpu_struct() {
  check_fn(
    |v| ray_plane_distance_fn(make_ray_fn(v.into_f32(), (v % val(2)).equals(0))),
    |v| {
      if v % 2 == 0 { v as f32 / 2. + 1. } else { -1. }
    },
  )
  .await;
}

/// the host shareable struct is the input and output of the function
#[pollster::test]
async fn fn_gpu_host_struct() {
  let input: Vec<_> = (0..8)
    .map(|i| {
      let f = i as f32;
      FnStd430Item {
        position: Vec3::new(f, f + 1., -f),
        weight: f * 0.5,
        flags: Vec2::new(i, i * 3),
        id: 100 + i,
        ..Default::default()
      }
    })
    .collect();
  let result = gpu_map(&input, |item| process_item_fn(item, val(2.))).await;

  let fields = |s: &FnStd430Item| (s.position, s.weight, s.flags, s.id);
  let result: Vec<_> = result.iter().map(fields).collect();
  let expect: Vec<_> = input
    .iter()
    .map(|s| {
      let p = s.position;
      let flags = Vec2::new(s.flags.y, s.flags.x);
      let position = Vec3::new(p.x * 2., p.y * 2., p.z * 2.);
      (position, s.weight + 2., flags, s.id + flags.x)
    })
    .collect();
  assert_eq!(result, expect);
}

/// the std140 struct with explicit padding members, the fields after the nested struct are
/// remapped when they are read from the parameter and when the returned struct is constructed
#[pollster::test]
async fn fn_gpu_padded_struct() {
  check_fn(
    |v| {
      let f = v.into_f32();
      let outer = ENode::<FnStd140Outer> {
        head: f,
        inner: ENode::<FnStd140Inner> { value: f + val(1.) }.construct(),
        tail: f + val(2.),
        direction: vec3_node((f, f + val(3.), val(1.))),
      };
      let extra = ENode::<FnStd140Inner> { value: f * val(2.) };
      let r = std140_shuffle_fn(outer.construct(), extra.construct()).expand();
      let d = r.direction;
      vec4_node((
        r.head,
        r.inner.expand().value,
        r.tail,
        d.x() + d.y() + d.z(),
      ))
    },
    |v| {
      let f = v as f32;
      Vec4::new(3. * f + 1., f + 2., f, 4. * f + 8.)
    },
  )
  .await;
  check_fn(
    |v| {
      let f = v.into_f32();
      let outer = ENode::<FnStd140Outer> {
        head: f,
        inner: ENode::<FnStd140Inner> { value: f + val(1.) }.construct(),
        tail: f + val(2.),
        direction: vec3_node((f, f + val(3.), val(1.))),
      };
      let extra = ENode::<FnStd140Inner> { value: f * val(2.) };
      let shuffled = std140_shuffle_fn(outer.construct(), extra.construct());
      let r = std140_through_pointer_fn(shuffled).expand();
      let d = r.direction;
      let d = d.x() * val(100.) + d.y() * val(10.) + d.z();
      vec4_node((r.head, r.inner.expand().value, r.tail, d))
    },
    |v| {
      // the shuffled struct is (3f + 1, f + 2, f, (2f, 2f + 6, 2))
      let f = v as f32;
      Vec4::new(3. * f + 1., 11. * f + 20., 3. * f + 1., 22. * f + 260.)
    },
  )
  .await;
}

/// the matrix parameters and return
#[pollster::test]
async fn fn_gpu_matrix() {
  check_fn(
    |v| {
      let f = v.into_f32();
      let m3 = mat3_node((
        vec3_node((val(1.), f, val(0.))),
        val(Vec3::new(0., 1., 2.)),
        vec3_node((f, val(0.), val(1.))),
      ));
      let m2x3 = (val(Vec3::new(1., 2., 3.)), vec3_node((f, val(0.), val(1.)))).into();
      let v = vec3_node((f, val(1.), val(2.)));
      matrix_combine_fn(m3, make_matrix_fn(f), m2x3, v)
    },
    |v| {
      let f = v as f32;
      // m3 * v = (3f, f * f + 1, 4), m4 * (v, 1) = (f * f + 1, f + 1, 4, 1)
      // transpose(m2x3) * v = (f + 8, f * f + 2), det(m3) = 1 + 2 * f * f
      Vec4::new(
        f * f + 3. * f + 5.,
        f * f + 2.,
        f + 6. - f * f,
        1. + 2. * f * f,
      )
    },
  )
  .await;
}

/// the array parameter and return
#[pollster::test]
async fn fn_gpu_array() {
  check_fn(
    |v| {
      let array = make_local_var::<[u32; 4]>();
      array.index(0).store(v);
      array.index(1).store(v * val(2));
      array.index(2).store(v + val(3));
      array.index(3).store(val(7));
      array_tail_sum(array_reverse_fn(array.load()), v % val(5))
    },
    |v| {
      let reversed = [7, v + 3, v * 2, v];
      reversed.iter().skip((v % 5) as usize).sum()
    },
  )
  .await;
}

/// the constants, the global constant and the local variables in the function body
#[pollster::test]
async fn fn_gpu_constants_and_locals() {
  check_fn(
    |v| constants_and_locals_fn(v.into_f32()),
    |v| Vec4::new(3.5, 6.25, 11., 3. * v as f32 + 4.),
  )
  .await;
}
