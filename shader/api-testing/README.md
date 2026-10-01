# Rendiation Shader API Testing

Tests of the shader EDSL (`rendiation-shader-api`) and its naga backend. All dependencies are
dev-dependencies, so the normal build of this crate is empty and never affects the compile time of
other crates. Only the GPU execution tests require a GPU adapter.

Run by `cargo test -p rendiation-shader-api-testing` (`--lib` skips the doctests).

The tests are grouped by topic, each topic is tested in two ways, plus the GPU execution when the
result values matter.

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
- `fake_storage_buffer(index)` / `fake_readonly_storage_buffer(index)`: a read_write or readonly
  storage buffer binding.
- `u32_heap_ptr(heap, offset)`: the typed pointer on a u32 heap, which is the pointer
  implementation of the combined buffer, to test the non native pointer.

The build time checks of the EDSL are tested by `#[should_panic]`.

## GPU execution

The validation only proves the shader is valid, when the result values matter (for example the
iterator semantics), run the shader on the GPU by `gpu_map(input, logic)` in `src/harness.rs`. It
dispatches one invocation for each input value, runs the `logic` on it and reads back the results
in the input order. The same shader is also built with the fake bindings and validated, so it is
covered by the regression check below. Compare the results with the cpu reference logic, and put
these tests in the topic file as `#[pollster::test]` async tests.

```rust
#[pollster::test]
async fn iter_gpu_count() {
  let input = [0, 1, 5];
  let result = gpu_map(&input, |v: Node<u32>| v.into_shader_iter().sum()).await;
  let expect: Vec<u32> = input.iter().map(|v| (0..*v).sum()).collect();
  assert_eq!(result, expect);
}
```

When the shader needs other bindings than one input and one output array (for example the std140
struct in the uniform buffer, the workgroup variables shared by the invocations, or the types
without rust type like the unsized struct), `gpu_run_raw_buffers` in `src/binding.rs` binds the
host bytes as any shader type in any buffer address space and reads back the outputs and the
buffers, `check_raw_buffers` validates the same logic with the fake bindings.

## Regression check of the generated shader

Every module that passes `validate` (the validation tests and the shaders of `gpu_map`) is written
to the directory given by the `WGSL_SNAPSHOT_DIR` environment variable, named
`<test path>_<module index in the test>.wgsl`. The naga WGSL writer does not support the ray query,
these modules are written as the naga IR (`.naga.txt`) instead. The output is deterministic, so a
change that should not affect the generated shader (for example a refactor of the naga backend) is
checked by comparing the output before and after the change. The output is not committed.

```sh
WGSL_SNAPSHOT_DIR=/tmp/wgsl_before cargo test -p rendiation-shader-api-testing --lib
# apply the change
WGSL_SNAPSHOT_DIR=/tmp/wgsl_after cargo test -p rendiation-shader-api-testing --lib
diff -r /tmp/wgsl_before /tmp/wgsl_after
```

Use empty directories, the files of the removed or renamed tests are not cleaned. The ignored tests
are not run, so their shaders are not in the output.

## When changing the EDSL

When an API bound is changed, add a compile fail case for the rejected usage and a validation case
for the valid usage.
