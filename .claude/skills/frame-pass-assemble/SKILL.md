---
name: frame-pass-assemble
description: >
  Reference for building multi-pass rendering frames in rendiation. Covers pass(), attachment(),
  render_ctx(), by(), by_if(), FrameCtx, PassContent, UseQuadDraw, and color/depth load-store
  operations. Use when wiring render passes in a frame — composing geometry, post-processing,
  MSAA resolves, and frame copies into a complete GPU frame.
  For the sub layer abstractions and impls (RenderComponent, ShaderHashProvider, ShaderPassBuilder, convenience wrappers)
  see fundamental-gpu-component-model.
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

Multi-pass rendering frame assembly in rendiation. The API is a functional/chainable builder for composing GPU render passes. Key files:

| File | Purpose |
|------|---------|
| [platform/graphics/webgpu/src/frame/pass.rs](platform/graphics/webgpu/src/frame/pass.rs) | `pass()`, `RenderPassDescription`, `ActiveRenderPass`, `PassContent`, `FrameRenderPass` |
| [platform/graphics/webgpu/src/frame/attachment.rs](platform/graphics/webgpu/src/frame/attachment.rs) | `attachment()`, `AttachmentDescriptor`, `PooledTextureKey` |
| [platform/graphics/webgpu/src/frame/mod.rs](platform/graphics/webgpu/src/frame/mod.rs) | `FrameCtx` |
| [platform/graphics/webgpu/src/frame/quad.rs](platform/graphics/webgpu/src/frame/quad.rs) | `UseQuadDraw`, `QuadDraw<T>` |
| [platform/graphics/webgpu/src/pass.rs](platform/graphics/webgpu/src/pass.rs) | `RenderTargetView`, `GPURenderPassCtx` |

For `RenderComponent`, `ShaderHashProvider`, `ShaderPassBuilder` and convenience wrappers, see `fundamental-gpu-component-model`.

## FrameCtx — the frame host

`FrameCtx` holds the GPU command encoder, texture pool, and frame state for one frame. All pass assembly happens inside code that receives `&mut FrameCtx`.

```rust
// Provided by the framework — you receive ctx, not create it.
// Key fields:
ctx.gpu          // &GPU
ctx.frame_size   // Size
// ctx.scope(f) creates a sub-scope with fresh hook memory
```


## pass() — creating a render pass

`pass(name)` returns a `RenderPassDescription` builder.

```rust
pass("my-pass")                     // RenderPassDescription
    .with_color(&target, op)        // push a color attachment(support multiple, in order)
    .with_depth(&depth, d_op, s_op) // set depth-stencil (optional)
    .render_ctx(ctx)                // start the GPU pass → ActiveRenderPass
```

### Color attachment operations

| Function | Behavior |
|----------|----------|
| `store_full_frame()` | Clear at load, store at end |
| `load_and_store()` | Preserve existing content, store at end |
| `load_once_and_discard()` | Preserve content, discard after pass |
| `clear_and_store(v)` | Clear to specific value, store at end |

### Depth attachment operations

Same functions, applied separately to depth and stencil ops:
```rust
.with_depth(&depth_view, load_and_store(), load_and_store())
//                        ^ depth op         ^ stencil op
```

### Resolving MSAA

```rust
pass("resolve")
    .with_color_and_resolve_target(
        &msaa_target,              // 4x MSAA
        load_once_and_discard(),   // consume MSAA, then discard
        &single_sample_target,     // resolve destination (1x)
    )
    .render_ctx(ctx);
```

### No-op pass with side effects

A pass with no `.by()` calls still runs — useful for MSAA resolves or clearing attachments.


## attachment() — allocating transient textures

`attachment()` returns an `AttachmentDescriptor` builder. Textures are allocated from a frame-persistent pool and automatically reused across frames.

```rust
attachment()
    .format(TextureFormat::Rgba16Float)  // default: Rgba8UnormSrgb
    .sample_count(4)                      // MSAA
    .sizer(ratio_sizer(0.5))              // half resolution
    .request(ctx)                         // → RenderTargetView
```

### Builder methods

| Method | Default | Description |
|--------|---------|-------------|
| `.format(f)` | `Rgba8UnormSrgb` | Texture format |
| `.sample_count(n)` | `1` | MSAA samples |
| `.sizer(f)` | identity | Size transform, e.g. `ratio_sizer(0.5)` |
| `.extra_usage(flags)` | — | Additional `TextureUsages` |
| `.use_hdr_if_enabled(bool)` | — | Switches to `Rgba16Float` when HDR is on |

### Pre-built shortcuts

```rust
attachment()           // default color
depth_attachment()     // default depth
```

### Reusing the same attachment across passes

```rust
let target = attachment().request(ctx);
// ... use &target in multiple passes within the same frame
```


## render_ctx() and ActiveRenderPass

`.render_ctx(ctx)` consumes the `RenderPassDescription`, starts the actual GPU render pass, and returns an `ActiveRenderPass`.

```rust
let active = pass("name")
    .with_color(&target, store_full_frame())
    .render_ctx(ctx);
// active: ActiveRenderPass
```

Use `.make_all_channel_and_depth_into_load_op()` on the description before `render_ctx()` to change all operations to `Load` (useful when you want to preserve the previous pass output without re-clearing).


## by() — rendering content into a pass

`.by(content)` renders a `PassContent` implementation into the active pass. Returns `Self` so calls can be chained.

```rust
pass("geometry")
    .with_color(&target, store_full_frame())
    .with_depth(&depth, load_and_store(), load_and_store())
    .render_ctx(ctx)
    .by(&mut draw_skybox)       // render skybox
    .by(&mut draw_geometry)     // render main geometry
    .by(&mut draw_transparent); // render transparents
// drop commits the pass
```

## by_if() — conditional rendering

`.by_if(&mut option)` renders only if `option` is `Some`.

```rust
let mut compose = pass("compose")
    .with_color(&final_target, load_and_store())
    .render_ctx(ctx)
    .by_if(&mut self.tonemap)      // optional tone mapping
    .by_if(&mut self.highlight);   // optional highlight overlay
```


## PassContent trait

Any type rendered via `.by()`. Two main paths:

```rust
// 1. QuadDraw<T> — fullscreen quad (most common for post-processing)
//    Any RenderComponent gets .draw_quad() for free via blanket impl:
my_post_processor.draw_quad()
my_shader.draw_quad_with_alpha_blending()
my_shader.draw_quad_with_blend(Some(my_blend_state))

// 2. Geometry draw — RenderComponent wrapped in RenderVec/RenderSlice
//    Draws with vertex/index buffers via DrawCommand
```

### PassContent vs RenderComponent — design split

The two traits address different levels of granularity:

- **`PassContent`** — the unit of **draw work** inside a pass. It represents a logically independent
  piece of business rendering logic. It receives the `FrameRenderPass` and may issue **multiple draw
  calls**, each backed by one or more `RenderComponent`s assembled via `RenderArray`/`RenderSlice`.
  It owns the decision of *what to draw and in what sequence* within the pass.

- **`RenderComponent`** — the unit of **shader logic** for a single draw call. It provides a reusable,
  independently cacheable piece of the GPU pipeline: shader definition, pipeline hash, and resource
  bindings. It has no concept of passes or draw commands — those are provided by the caller.
  `RenderSlice`/`RenderArray` can also serve as `RenderComponent` combinators, nesting multiple
  components into one through the onion model.

A `PassContent` implementation typically assembles a `DefaultPassDispatcher` (per-pass state) plus
one or more `RenderComponent`s, calls `RenderComponent::render()` for each draw, and controls the
draw sequence. `QuadDraw<T>` is one example of `PassContent` — it wraps a single `RenderComponent`
in a fullscreen quad — but any multi-draw pattern (geometry pass with skybox + main + transparents,
OIT with depth pre-pass + color + resolve) is also a `PassContent`.

For details on the component model (`ShaderHashProvider`, `ShaderPassBuilder`, `GraphicsShaderProvider`,
convenience wrappers, onion middleware model), see `fundamental-gpu-component-model`.


## Complete frame example

From [application/viewer-content/src/rendering/frame_viewport.rs](application/viewer-content/src/rendering/frame_viewport.rs):

```rust
fn render(&mut self, ctx: &mut FrameCtx) {
    // Allocate attachments
    let scene_result = attachment()
        .sample_count(4)
        .request(ctx);
    let depth = depth_attachment()
        .sample_count(4)
        .request(ctx);
    let simple_sample = attachment().request(ctx); // 1x for resolve

    // Pass 1: render geometry with MSAA
    pass("scene")
        .with_color(&scene_result, store_full_frame())
        .with_depth(&depth, load_and_store(), load_and_store())
        .render_ctx(ctx)
        .by(&mut self.skybox)
        .by(&mut self.geometry);

    // Pass 2: resolve MSAA
    pass("resolve")
        .with_color_and_resolve_target(
            &scene_result,
            load_once_and_discard(),
            &simple_sample,
        )
        .render_ctx(ctx);

    // Pass 3: post-process
    let final_target = attachment().request(ctx);
    pass("post")
        .with_color(&final_target, store_full_frame())
        .render_ctx(ctx)
        .by(&mut PostProcess {
            input: simple_sample,
        }.draw_quad());
}
```


## Key types quick reference

| Type | Role |
|------|------|
| `FrameCtx` | Per-frame GPU state, encoder, pool, memory |
| `RenderPassDescription` | Pass builder: name + color/depth attachments |
| `ActiveRenderPass` | Running pass: chain `.by()` calls |
| `AttachmentDescriptor` | Texture allocation builder |
| `RenderTargetView` | Allocated texture handle (from pool) |
| `PassContent` | Trait: anything renderable into a pass |
| `GPURenderPassCtx` | Per-pass render state (bindings, pipeline) |
| `RenderComponent` | Shader + hash + setup = renderable entity |
| `UseQuadDraw` | Blanket: any T → `QuadDraw<T>` for fullscreen quad |
