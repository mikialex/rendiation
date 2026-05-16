---
name: shader-edsl-graphics
description: >
  Graphics pipeline reference for the rendiation shader EDSL. Covers GraphicsShaderProvider,
  vertex/fragment stages, semantics (built-in and custom), resource binding (textures, buffers,
  samplers), render targets, and common graphics recipes. Use when building vertex+fragment
  shader pipelines. Depends on shader-edsl-core for the stage-agnostic language primitives
  and shader-edsl-binding-and-typed-container for resource binding.
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

Rendiation graphics pipeline reference. For the core language see `shader-edsl-core`. For resource binding see `shader-edsl-binding-and-typed-container`. For compute pipelines see `shader-edsl-compute`.

```rust
use rendiation_shader_api::*;
```


## Graphics Pipeline Template

```rust
impl GraphicsShaderProvider for MyPass {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.fragment(|builder, binding| {
            // 1. Bind resources
            let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&self.input_texture);
            let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

            // 2. Query semantics (dependency injection)
            let uv: Node<Vec2<f32>> = builder.query::<FragmentUv>();

            // 3. Write shader logic
            let color = tex.sample(sampler, uv);

            // 4. Output
            builder.store_fragment_out(0, color);
        });
    }
}
```

**Key methods**:

- `builder.vertex(|builder, binding| { ... })` — enter vertex stage
- `builder.fragment(|builder, binding| { ... })` — enter fragment stage
- `binding.bind_by(&resource)` — bind resource (texture/buffer)
- `binding.bind_single_by(&resource)` — bind a simple resource
- `builder.store_fragment_out(slot, value)` — write fragment output
- `builder.store_fragment_out_vec4f(slot, vec4)` — write vec4 to 4xf32 output (common convenience)

### Full vertex + fragment example

```rust
fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
        let pos = builder.query::<GeometryPosition>();
        let mvp = builder.query::<CameraViewNoneTranslationProjectionMatrix>();
        builder.set_vertex_out::<GeometryUV>(builder.query::<GeometryUV>());
        builder.register::<ClipPosition>(mvp * (pos, val(1.0)).into());
    });
    builder.fragment(|builder, binding| {
        let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();
        let tex = binding.bind_by(&self.tex);
        let smp = binding.bind_by(&ImmediateGPUSamplerViewBind);
        builder.store_fragment_out_vec4f(0, tex.sample(smp, uv));
    });
}
```

### Register vertex buffer (CPU side)

```rust
builder.register_vertex::<CommonVertex>(VertexStepMode::Vertex);
```


## Vertex Input & Semantics

### Define vertex input layout

```rust
#[repr(C)]
#[derive(rendiation_shader_api::ShaderVertex, Clone, Copy, Debug)]
pub struct CommonVertex {
    #[semantic(GeometryPosition)]
    pub position: Vec3<f32>,
    #[semantic(GeometryNormal)]
    pub normal: Vec3<f32>,
    #[semantic(GeometryUV)]
    pub uv: Vec2<f32>,
}
```

`#[semantic(X)]` associates a field with a built-in semantic. `#[derive(ShaderVertex)]` generates the `ShaderVertexInProvider` impl.

### Query vertex inputs

```rust
let pos: Node<Vec3<f32>> = builder.query::<GeometryPosition>();
let normal: Node<Vec3<f32>> = builder.query::<GeometryNormal>();
```

### Set vertex outputs

```rust
builder.set_vertex_out::<GeometryUV>(uv);
builder.set_vertex_out_with_given_interpolate::<FragmentColor>(color);

// Built-in vertex output — must write (x, y, z, w)
builder.register::<ClipPosition>(clip_pos);
```

### Custom semantics

```rust
only_vertex!(MyVertexData, Vec4<f32>);    // vertex stage only
only_fragment!(MyFragData, Vec3<f32>);    // fragment stage only
both!(MySharedData, f32);                 // vertex + fragment shared
```

Usage:

```rust
builder.query::<MyVertexData>();         // read
builder.register::<MySharedData>(val);   // write
```


## Fragment Shader Patterns

### Core methods

```rust
// Query semantic (panics if not found)
let uv: Node<Vec2<f32>> = builder.query::<FragmentUv>();

// Safe query (returns Option)
let color = builder.try_query::<FragmentColor>();

// Query from fragment, fall back to vertex interpolation
let norm: Node<Vec3<f32>> = builder.query_or_interpolate_by::<FragmentRenderNormal, VertexRenderNormal>();

// Query or insert default
let val = builder.query_or_insert_default::<FragmentUv>();

// Register a semantic value
builder.register::<FragmentRenderNormal>(normal);

// Output
builder.store_fragment_out(0, color);          // output to slot 0
builder.store_fragment_out_vec4f(0, vec4);     // vec4 output (common)

// Multiple render targets
builder.define_out_by(channel(format));        // declare new output slot
builder.store_fragment_out(1, another_color);  // write to slot 1

// Special operations
builder.discard();                              // discard fragment
builder.register::<FragmentDepthOutput>(depth); // write depth

// Convenience methods
builder.get_or_compute_fragment_uv();           // auto-get or compute UV
builder.get_or_compute_fragment_normal();       // auto-get or compute normal
```

### Fragment output patterns

```rust
// Single output
builder.store_fragment_out_vec4f(0, color);

// Multi-output (e.g. deferred shading)
builder.define_out_by(channel(TextureFormat::Rgba8Unorm));      // slot 0
builder.define_out_by(channel(TextureFormat::Rgba16Float));     // slot 1
builder.store_fragment_out_vec4f(0, albedo);
builder.store_fragment_out_vec4f(1, normal_and_roughness);
```

## Semantics Quick Reference

### Vertex Input (geometry data, CPU-uploaded)

| Semantic | Rust type |
|----------|-----------|
| `GeometryPosition` | `Vec3<f32>` |
| `GeometryPosition2D` | `Vec2<f32>` |
| `GeometryNormal` | `Vec3<f32>` |
| `GeometryTangent` | `Vec4<f32>` |
| `GeometryUV` (= `GeometryUVChannel<0>`) | `Vec2<f32>` |
| `GeometryUVChannel<I>` | `Vec2<f32>` |
| `GeometryColor` | `Vec3<f32>` |
| `GeometryColorWithAlpha` | `Vec4<f32>` |
| `JointIndexChannel<I>` | `Vec4<u32>` |
| `WeightChannel<I>` | `Vec4<f32>` |

### Vertex Built-in

| Semantic | Type | Description |
|----------|------|-------------|
| `VertexIndex` | `u32` | gl_VertexIndex |
| `VertexInstanceIndex` | `u32` | gl_InstanceIndex |

### Vertex Output

| Semantic | Type | Description |
|----------|------|-------------|
| `ClipPosition` | `Vec4<f32>` | Must write (x, y, z, w) |
| `VertexRenderPosition` | `Vec3<f32>` | World-space position |
| `VertexRenderNormal` | `Vec3<f32>` | World-space normal |

### Fragment Input (interpolated from Vertex Output)

| Semantic | Type | Description |
|----------|------|-------------|
| `FragmentFrontFacing` | `bool` | Front facing |
| `FragmentPosition` | `Vec4<f32>` | (x,y) = framebuffer coords |
| `FragmentSampleIndex` | `u32` | |
| `FragmentSampleMaskInput` | `u32` | |

### Fragment Shared (Vertex writes, Fragment reads)

| Semantic | Type | Description |
|----------|------|-------------|
| `FragmentUv` | `Vec2<f32>` | Texture coordinates |
| `FragmentRenderPosition` | `Vec3<f32>` | World-space position |
| `FragmentRenderNormal` | `Vec3<f32>` | World-space normal |
| `FragmentColor` | `Vec3<f32>` | Vertex color |

### Fragment Output

| Semantic | Type | Description |
|----------|------|-------------|
| `FragmentDepthOutput` | `f32` | Depth write |
| `FragmentSampleMaskOutput` | `u32` | |

### Render Context (auto-provided)

| Semantic | Type | Description |
|----------|------|-------------|
| `ViewportRenderBufferSize` | `Vec2<f32>` | Viewport resolution |
| `TexelSize` | `Vec2<f32>` | 1 / resolution |
| `CameraProjectionMatrix` | `Mat4<f32>` | Projection matrix |
| `CameraProjectionInverseMatrix` | `Mat4<f32>` | Inverse projection |
| `CameraWorldNoneTranslationMatrix` | `Mat4<f32>` | Camera no-translation matrix |
| `CameraWorldPositionHP` | `HighPrecisionTranslation` | Camera position (high precision) |
| `CameraViewNoneTranslationProjectionMatrix` | `Mat4<f32>` | View-Proj matrix |
| `CameraViewNoneTranslationProjectionInverseMatrix` | `Mat4<f32>` | Inverse View-Proj |
| `WorldPositionHP` | `HighPrecisionTranslation` | Object world position (high precision) |
| `WorldNoneTranslationMatrix` | `Mat4<f32>` | Object world matrix |
| `WorldNormalMatrix` | `Mat3<f32>` | Normal matrix |

### Lighting / Rendering

| Semantic | Type | Description |
|----------|------|-------------|
| `ColorChannel` | `Vec3<f32>` | Base color |
| `EmissiveChannel` | `Vec3<f32>` | Emissive |
| `AlphaChannel` | `f32` | Alpha |
| `HDRLightResult` | `Vec3<f32>` | HDR light result (fragment only) |
| `LDRLightResult` | `Vec3<f32>` | LDR light result (fragment only) |
| `ShouldUsePreSetLDRResult` | `bool` | Preset LDR (fragment only) |
| `DefaultDisplay` | `Vec4<f32>` | Default debug display |


## Common Patterns (Recipes)

### 6.1 Texture bind + sample

```rust
let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&self.input);
let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
let uv: Node<Vec2<f32>> = builder.query::<FragmentUv>();
let color = tex.sample(sampler, uv);
builder.store_fragment_out_vec4f(0, color);
```

### 6.2 Uniform struct

```rust
let uniform: BindingNode<ShaderUniformBuffer<Params>> = binding.bind_by(&self.params);
let f = uniform.load().expand();
let value = f.field * val(2.0);
```

### 6.3 Post-processing pass (fragment only)

```rust
builder.fragment(|builder, binding| {
    let uv = builder.query::<FragmentUv>();
    let tex = binding.bind_by(&self.input);
    let smp = binding.bind_by(&ImmediateGPUSamplerViewBind);
    let color = tex.sample(smp, uv);
    builder.store_fragment_out_vec4f(0, color);
});
```

### 6.4 Multiple Render Targets

```rust
builder.define_out_by(channel(TextureFormat::Rgba8Unorm));     // slot 0
builder.define_out_by(channel(TextureFormat::Rgba16Float));    // slot 1
builder.store_fragment_out_vec4f(0, albedo);
builder.store_fragment_out_vec4f(1, normal_roughness);
```

### 6.5 SSAO-style: iterate + accumulate

```rust
let result = samples
    .into_shader_iter()
    .clamp_by(sample_count)
    .map(|(_, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        let s = sample.load();
        // process sample ...
        val(0.0) // return contribution
    })
    .sum();
```

### 6.6 Dynamic array iteration (e.g. blur weights)

```rust
let weight_count: Node<u32> = binding.bind_by(&self.count).load().x();
let sum = weights
    .into_shader_iter()
    .clamp_by(weight_count)
    .map(|(i, weight): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        let w = weight.load();
        let sample_uv = uv + size * direction * i.into_f32();
        tex.sample(sampler, sample_uv) * w
    })
    .sum();
```


## Gotchas (Graphics-specific)

### Fragment Output

- Fragment output slots must be **declared before use** (auto-declared on first `store_fragment_out`)
- Multiple output slots require explicit `define_out_by(channel(format))`

### Vertex → Fragment sync

- Vertex outputs auto-sync to Fragment inputs (same `both!` semantic)
- Use `builder.query_or_interpolate_by::<FragType, VertType>()` to declare the dependency


## Reference Examples

| Example | File |
|---------|------|
| PBR material (GLES) | [scene/rendering/gpu-gles/src/material/mr.rs](scene/rendering/gpu-gles/src/material/mr.rs) |
| SSAO | [content/texture/gpu-process/src/ssao.rs](content/texture/gpu-process/src/ssao.rs) |
| FXAA | [content/texture/gpu-process/src/fxaa.rs](content/texture/gpu-process/src/fxaa.rs) |
| Linear blur | [content/texture/gpu-process/src/blur.rs](content/texture/gpu-process/src/blur.rs) |
| Tone mapping | [content/texture/gpu-process/src/tonemap.rs](content/texture/gpu-process/src/tonemap.rs) |
| Grid ground | [application/viewer-content/src/rendering/grid_ground.rs](application/viewer-content/src/rendering/grid_ground.rs) |
| Wide line | [extension/wide-line/src/draw.rs](extension/wide-line/src/draw.rs) |
| Font rendering | [extension/text-3d/src/slug_shader.rs](extension/text-3d/src/slug_shader.rs) |
