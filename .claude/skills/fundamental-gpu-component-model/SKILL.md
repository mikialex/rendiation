---
name: fundamental-gpu-component-model
description: >
  Reference for the composable GPU component model in rendiation. Covers RenderComponent,
  ShaderHashProvider, ShaderPassBuilder, GraphicsShaderProvider, and convenience wrappers
  (RenderVec, RenderSlice, RenderArray, OptionRender, BindingController). Use when defining
  a renderable entity that participates in the pipeline cache, resource binding, and draw
  dispatch system.
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

The composable GPU component model is the core abstraction for renderable entities in rendiation.
A type becomes renderable by implementing three traits — `ShaderHashProvider`, `GraphicsShaderProvider`,
and `ShaderPassBuilder`. The framework auto-derives `RenderComponent` for any type that satisfies all three,
giving it a complete render path: pipeline caching, resource binding, and draw dispatch.

Key files:

| File | Purpose |
|------|---------|
| [platform/graphics/webgpu/src/rendering.rs](platform/graphics/webgpu/src/rendering.rs) | `RenderComponent`, wrappers |
| [platform/graphics/webgpu/src/pass.rs](platform/graphics/webgpu/src/pass.rs) | `ShaderPassBuilder`, `GPURenderPassCtx`, `GPURenderPass` |
| [platform/graphics/webgpu/src/device.rs](platform/graphics/webgpu/src/device.rs) | `ShaderHashProvider`, `PipelineHasher`, pipeline cache |
| [platform/graphics/webgpu/src/frame/pass_base.rs](platform/graphics/webgpu/src/frame/pass_base.rs) | `DefaultPassDispatcher` |
| [shader/api/src/graphics/mod.rs](shader/api/src/graphics/mod.rs) | `GraphicsShaderProvider` |


## The three super-traits

```rust
// Auto-implemented for any T: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder
pub trait RenderComponent: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder { ... }
```

A type does **not** explicitly implement `RenderComponent`. It implements the three constituent traits,
and the blanket impl provides `RenderComponent` automatically.


## ShaderHashProvider — pipeline caching key

**File**: [platform/graphics/webgpu/src/device.rs](platform/graphics/webgpu/src/device.rs)

```rust
pub trait ShaderHashProvider {
    fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
    fn hash_type_info(&self, hasher: &mut PipelineHasher);
    fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
        self.hash_type_info(hasher);
        self.hash_pipeline(hasher);
    }
}
```

Two hashing hooks:

- `hash_type_info` — **required**. Hashes the structural identity of the type (normally `TypeId`).
  This ensures different component types never collide in the cache even if they hash the same data.
- `hash_pipeline` — optional default, but **must be overridden whenever any data influences what
  shader code is generated in `build()` / `post_build()`**.

The resulting `u64` hash is used as the key in `GPUDevice.render_pipeline_cache: HashMap<u64, GPURenderPipeline>`.
A cache miss triggers shader compilation via `GraphicsShaderProvider::build_self`.

### The `hash_pipeline` correctness rule

**`hash_pipeline` must hash every piece of data that can influence the compiled `GPURenderPipeline`.**

A `GPURenderPipeline` bakes together the compiled shader program AND all fixed-function pipeline
state (blend modes, depth-stencil, color target formats, sample count, primitive topology, etc.).
Both categories must be covered by the hash:

- **Shader code** — Since `build()` / `post_build()` are procedural Rust code that use the EDSL
  to emit shader logic, any Rust control flow (`if`, `match`, loops, config flags) that alters
  the generated shader output must be hashed.
- **Pipeline state** — Anything set on the `ShaderRenderPipelineBuilder` inside `build()` or
  `post_build()`: color target format, depth-stencil config, sample count, blend state,
  primitive state, etc.

If two component instances produce different shader logic or different pipeline state but return
the same hash, the cache will return the wrong pipeline — causing draw corruption, binding layout
mismatches, or WGPU validation errors.

A minimal checklist:

- State enums / flags that branch inside `build()` or `post_build()`
- Color/depth format and sample count registered via `builder.define_out_by()`
- Blend state, depth-stencil ops, primitive state (cull mode, etc.)
- Any runtime configuration that affects binding descriptor generation

If in doubt, hash it. A false cache miss costs one compilation; a false cache hit corrupts rendering.

### Helper macro: `shader_hash_type_id!`

```rust
// Most common pattern — hash TypeId::of::<Self>() for hash_type_info
impl ShaderHashProvider for MyComponent {
    shader_hash_type_id!();
    fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.some_config.hash(hasher);
    }
}
```

### Blanket implementations

- `impl ShaderHashProvider for ()` — no-op (both `hash_type_info` and `hash_pipeline` do nothing)
- `impl<T: ShaderHashProvider> ShaderHashProvider for &T` — delegates to `T`


## ShaderPassBuilder — resource binding setup

**File**: [platform/graphics/webgpu/src/pass.rs](platform/graphics/webgpu/src/pass.rs)

```rust
pub trait ShaderPassBuilder {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {}
    fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {}

    fn setup_pass_self(&self, ctx: &mut GPURenderPassCtx) {
        self.setup_pass(ctx);
        self.post_setup_pass(ctx);
    }
}
```

- `setup_pass` — called **before** the pipeline is bound. Bind textures, buffers, samplers here via `ctx.binding`.
- `post_setup_pass` — called **after** the pipeline is bound. Rarely needed; used for bindings that depend on
  knowing the pipeline layout first.

`ctx` is `&mut GPURenderPassCtx`, which provides:

```rust
pub struct GPURenderPassCtx {
    pub pass: GPURenderPass,        // the wgpu render pass (Deref to gpu::RenderPass<'static>)
    pub gpu: GPU,                   // device + queue access
    pub binding: BindingBuilder,    // accumulate bind groups for this pass
    incremental_vertex_binding_index: u32,
    pub enable_bind_check: bool,
}
```

### Binding contract: setup must match build, post_setup must match post_build

The binding declarations in `ShaderPassBuilder` and the shader binding declarations in
`GraphicsShaderProvider` form a strict contract that **must always be consistent**:

| Setup side | Build side |
|-----------|------------|
| `setup_pass()` | `build()` |
| `post_setup_pass()` | `post_build()` |

The rules apply to **both** pairs equally:

- **Order** — resources must be bound in the exact same sequence as declared on the shader side.
  Binding group index N maps to shader binding N.
- **Type** — each binding's type (texture, uniform buffer, storage buffer, sampler) must match the
  corresponding shader declaration.
- **Dynamic branches** — if `build()` or `post_build()` uses Rust control flow to conditionally
  declare bindings, the corresponding `setup_pass()` or `post_setup_pass()` must follow the
  **same control flow with the same conditions**.

Any mismatch causes WGPU validation errors at draw time. When bindings vary dynamically, the dynamic
factors must be hashed in `hash_pipeline` — otherwise a cached pipeline with a different binding
layout will be reused, causing binding index or type mismatches.

### Typical implementation

```rust
impl ShaderPassBuilder for MyEffect {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        // Order and types must match exactly what build() declares:
        ctx.binding.bind(&self.texture);   // matches binding 0 in build()
        ctx.binding.bind(&self.sampler);   // matches binding 1 in build()
        ctx.binding.bind(&self.uniform);   // matches binding 2 in build()
    }
}
```

Blanket: `impl ShaderPassBuilder for ()`, `impl<T: ShaderPassBuilder> ShaderPassBuilder for &T`.


## GraphicsShaderProvider — shader definition

**File**: [shader/api/src/graphics/mod.rs](shader/api/src/graphics/mod.rs)

```rust
pub trait GraphicsShaderProvider {
    fn build(&self, _builder: &mut ShaderRenderPipelineBuilder) {}
    fn post_build(&self, _builder: &mut ShaderRenderPipelineBuilder) {}

    fn build_self(
        &self,
        api_builder: &dyn Fn(ShaderStage) -> DynamicShaderAPI,
        info: Arc<GPUInfo>,
        checks: ShaderRuntimeChecks,
    ) -> Result<ShaderRenderPipelineBuilder, Vec<ShaderBuildError>> { ... }

    fn debug_label(&self) -> String { ... }
}
```

- `build` — register the vertex/fragment shader stages. Called first.
- `post_build` — add final defaults (e.g. auto-write white to fragment output 0). Called after `build`.
- `build_self` — orchestrates the full build: creates builder, calls `build`, calls `post_build`, collects errors.
- `debug_label` — auto-derived from the type's short name via `disqualified::ShortName`.

See `shader-edsl-graphics` skill for details on how to write the shader code inside `build`.


## How RenderComponent::render works

When `RenderComponent::render(ctx, draw_command)` is called, the framework executes this sequence:

1. **Hash** — calls `self.hash_pipeline_with_type_info(hasher)` to get a cache key
2. **Pipeline lookup or build** — `ctx.gpu.device.get_or_cache_create_render_pipeline(hasher, |device| { ... })`
   - On cache miss: calls `self.build_self(...)` → triggers `build()` then `post_build()` (onion forward then reverse)
   - On cache hit: returns the cached pipeline
3. **Reset state** — clears `ctx.binding` and resets vertex binding index
4. **Set pipeline** — binds the compiled `GPURenderPipeline` on the wgpu render pass
5. **Binding check** — if enabled, validates accumulated bind group layouts against the pipeline's expected layouts
6. **Setup pass** — calls `self.setup_pass_self(ctx)` which runs `setup_pass()` then `post_setup_pass()`.
   For `RenderSlice` this is where the onion traversal happens: `setup_pass` forward (A→B→C),
   then `post_setup_pass` reverse (C→B→A).
7. **Flush bindings** — `ctx.binding.setup_render_pass(&mut ctx.pass, ...)` commits all accumulated bind groups to the wgpu render pass
8. **Draw** — `ctx.pass.draw_by_command(draw_command)` issues the GPU draw call

This all happens inside a single `.by()` call on an `ActiveRenderPass`.


## The onion model — middleware composition via `RenderSlice`

The `pre` (forward) / `post` (reverse) method pairs in `GraphicsShaderProvider` and `ShaderPassBuilder`
are designed to compose components in an **onion (middleware) pattern**. When components `[A, B, C]` are
wrapped in a `RenderSlice`, the execution order is:

```
    ┌──────────────────────────┐
    │  A                       │
    │    ┌──────────────────┐  │
    │    │ B                │  │
    │    │   ┌──────────┐   │  │
    │    │   │ C (core) │   │  │
    │    │   └──────────┘   │  │
    │    └──────────────────┘  │
    └──────────────────────────┘
```

**Shader building** (`build` then `post_build`):
```
build:      A → B → C    (outer layers register first)
post_build: C → B → A    (inner layers finalize first)
```

**Pass setup** (`setup_pass` then `post_setup_pass`):
```
setup_pass:      A → B → C    (outer layers bind first)
post_setup_pass: C → B → A    (inner layers clean up / override first)
```

A outer component can wrap or override the behavior of inner components:

- In `build`, A registers a base pass configuration, B adds to it, C provides the final shader logic.
- In `post_build`, C adds its output defaults, B wraps them, A applies final global overrides (e.g. auto-write white).
- In `setup_pass`, A binds global resources (viewport uniform), B binds its textures, C binds its specific data.
- In `post_setup_pass`, C could emit post-draw cleanup, then B, then A.

The forward pass builds context from outermost → innermost; the reverse pass unwinds innermost → outermost,
just like middleware stacks in HTTP frameworks or Rust's `tower::Service` layers.

The convenience wrappers (`RenderSlice`, `RenderArray`, `RenderVec`) are the combinators that enable this
pattern — they call each element in forward order for the `pre` method, then in **reverse** order for the
`post` method. `RenderSlice` is the canonical implementation; `RenderArray` and `RenderVec` delegate to it.


## DefaultPassDispatcher — per-pass base component

**File**: [platform/graphics/webgpu/src/frame/pass_base.rs](platform/graphics/webgpu/src/frame/pass_base.rs)
**Constructor**: `default_dispatcher(pass: &FrameRenderPass, reversed_depth: bool) -> DefaultPassDispatcher`

Every render pass typically includes this as its first component. It:

- Binds the `pass_info` uniform buffer (viewport size, texel size) so shaders can query `ViewportRenderBufferSize` / `TexelSize`
- Registers the render target formats, depth-stencil state, and multisample count
- Optionally auto-writes white to fragment output 0 (`auto_write: bool` — default `true`, set to `false` when you write your own output)

Usage inside `PassContent::render`:

```rust
fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut base = default_dispatcher(pass, false);
    base.auto_write = false;  // I'll write my own output
    let components: [&dyn RenderComponent; 3] = [&base, &self.quad, &self.content];
    RenderArray(components).render(&mut pass.ctx, QUAD_DRAW_CMD);
}
```


## Putting it all together

A typical renderable component:

```rust
// 1. Define the shader
impl GraphicsShaderProvider for MyEffect {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.fragment(|builder, binding| {
            let uv = builder.query::<FragmentUv>();
            let tex = binding.bind_by(&self.input);
            let smp = binding.bind_by(&ImmediateGPUSamplerViewBind);
            builder.store_fragment_out_vec4f(0, tex.sample(smp, uv));
        });
    }
}

// 2. Provide a pipeline cache key
impl ShaderHashProvider for MyEffect {
    shader_hash_type_id!();
}

// 3. Bind resources at draw time
impl ShaderPassBuilder for MyEffect {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.input);
        ctx.binding.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    }
}

// RenderComponent is now auto-implemented — MyEffect is renderable.
// Use it in a pass (see frame-pass-assemble skill for pass() / .by() details):
pass("effect")
    .with_color(&target, store_full_frame())
    .render_ctx(ctx)
    .by(&mut MyEffect { input }.draw_quad());
```

### DrawCommand variants

`DrawCommand` controls how vertices are issued:

| Variant | Use |
|---------|-----|
| `DrawCommand::Array { vertices, instances }` | Non-indexed draw (used by fullscreen quads: `0..4, 0..1`) |
| `DrawCommand::Indexed { indices, instances, base_vertex }` | Indexed draw with vertex/index buffers |
| `DrawCommand::Indirect { ... }` | GPU-driven indirect draw |
| `DrawCommand::MultiIndirect { ... }` | Multi-draw indirect |
| `DrawCommand::MultiIndirectCount { ... }` | Multi-draw indirect with count buffer |
