# Rendiation Shader API Testing

Tests of the shader EDSL (`rendiation-shader-api`) and its naga backend. No GPU is required, all
dependencies are dev-dependencies, so the normal build of this crate is empty and never affects the
compile time of other crates.

Run by `cargo test -p rendiation-shader-api-testing` (`--lib` skips the doctests).

The tests are grouped by topic, each topic is tested in two ways.

## Compile fail

`src/compile_fail/<topic>.rs` (only compiled for doctest) contains WGSL invalid code that must be
rejected by the rust type system. Each case is a `compile_fail,E0xxx` doctest attached to a unit
struct named after the case, it is compiled independently and the error code is checked (nightly
only).

```rust
/// cube texture does not support sampling offset
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(tex: BindingNode<ShaderTextureCube>, s: BindingNode<ShaderSampler>) {
///   let call = tex.build_sample_call(s, zeroed_val::<Vec3<f32>>());
///   call.with_offset(Vec2::new(1, 1));
/// }
/// ```
pub struct CubeTextureOffset;
```

Keep each case minimal so the error code can only come from the tested usage, and keep a valid twin
of it in the validation tests.

## Validation

`src/<topic>.rs` contains WGSL valid code, built by `check_compute(|builder| ..)` or
`check_graphics(|builder| ..)` (vertex and fragment, with a fake `GPUInfo`), which runs the naga
validation. Naga is only used as the validator here, do not assert the generated shader source.

Helpers in `src/harness.rs`:

- `keep(node)`: make the expression used by a statement.
- `runtime_values(builder)`: the values only known at runtime, so the expressions are not constant.
- `fake_binding(index)`: create texture or sampler bindings without any GPU resource container.
- `fake_storage_texture_binding(index, format)`: the storage format is not expressed in the shader
  type, so it must be given to match the channel type.
- `fake_storage_buffer(index)`: a read_write storage buffer binding.
- `u32_heap_ptr(heap, offset)`: the typed pointer on a u32 heap, which is the pointer
  implementation of the combined buffer, to test the non native pointer.

The build time checks of the EDSL are tested by `#[should_panic]`.

## When changing the EDSL

When an API bound is changed, add a compile fail case for the rejected usage and a validation case
for the valid usage.
