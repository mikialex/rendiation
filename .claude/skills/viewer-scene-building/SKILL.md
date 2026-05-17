---
name: viewer-scene-building
description: >
  Practical recipes for building 3D test scenes in the rendiation viewer application.
  Covers mesh generation from ParametricSurface (build_attributes_mesh, triangulate_parametric),
  material creation patterns for all material types, wiring scene models, light setup,
  transforms, and the test-content module pattern.
  For the underlying scene data model (entity types, components, foreign keys, SceneWriter API),
  see scene-core-structure. For the relational database layer, see database-schema.
metadata:
  version: "2.1"
  updated: "2026-05-17"
  depends: [scene-core-structure, database-schema]
---

Practical scene-building recipes for the rendiation viewer. For the scene data model and `SceneWriter` API reference, see [[scene-core-structure]]. For the database layer, see [[database-schema]].

Key files used here:

| File | Purpose |
|------|---------|
| [application/viewer/src/viewer/default_scene.rs](application/viewer/src/viewer/default_scene.rs) | Canonical scene setup patterns |
| [application/viewer/src/viewer/test_content/](application/viewer/src/viewer/test_content/) | Test scene functions |
| [application/viewer/src/viewer/test_content/widen_line.rs](application/viewer/src/viewer/test_content/widen_line.rs) | Wide line test examples |
| [content/mesh/generator/src/lib.rs](content/mesh/generator/src/lib.rs) | `build_attributes_mesh`, `AttributesMeshBuilder` |
| [content/mesh/generator/src/builder/mod.rs](content/mesh/generator/src/builder/mod.rs) | `triangulate_parametric`, `TessellationConfig` |
| [content/mesh/generator/src/parametric.rs](content/mesh/generator/src/parametric.rs) | `ParametricSurface` trait |

## Imports

```rust
use rendiation_algebra::*;
use rendiation_mesh_generator::*;
use crate::*;
```

## Mesh generation

### From a ParametricSurface

```rust
let mesh = build_attributes_mesh(|builder| {
    builder.triangulate_parametric(
        &surface,
        TessellationConfig { u: 32, v: 32 },
        true,  // keep_grouping: push a new draw group
    );
})
.build();
```

- `build_attributes_mesh` creates an `AttributesMeshBuilder`, runs the closure, finishes the mesh, collects primitives into `AttributesMeshData`.
- `.build()` on `AttributesMeshData` produces a GPU-ready mesh handle.
- `TessellationConfig.u` / `.v` control subdivision density.

### Multiple faces in one mesh

```rust
let mesh = build_attributes_mesh(|builder| {
    for face in cube.make_faces() {
        builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
    }
})
.build();
```

### Writing mesh data

```rust
let mesh = writer.write_solid_attribute_mesh(attribute_mesh).mesh;
```

For mesh tracked by the asset system:
```rust
let mesh = writer
    .write_solid_attribute_mesh_data_uri(attribute_mesh, mesh_source)
    .mesh;
```

## Material creation patterns

### PBR Specular-Glossiness (simplest for colored objects)

```rust
let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::new(0.8, 0.3, 0.3),
    ..Default::default()
}
.write(&mut writer.pbr_sg_mat_writer);
let material = SceneMaterialDataView::PbrSGMaterial(material);
```

### PBR Specular-Glossiness with alpha blending

```rust
let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: color,
    alpha: AlphaConfigDataView {
        alpha_mode: AlphaMode::Blend,
        alpha: 0.5,
        ..Default::default()
    },
    ..Default::default()
}
.write(&mut writer.pbr_sg_mat_writer);
```

### PBR Metallic-Roughness with texture

```rust
let material = PhysicalMetallicRoughnessMaterialDataView {
    base_color: Vec3::splat(0.8),
    base_color_texture: Some(texture_handle),
    roughness: 0.1,
    metallic: 0.8,
    ..Default::default()
}
.write(&mut writer.pbr_mr_mat_writer);
let material = SceneMaterialDataView::PbrMRMaterial(material);
```

### OccStyle (CAD-style)

```rust
// See extension/occ-style-material for the full type paths
use rendiation_occ_style_material::*;

let mut occ_writer = global_entity_of::<OccStyleMaterialEntity>().entity_writer();
let occ_material = occ_writer.new_entity(|w| {
    let w = w
        .write::<OccStyleMaterialDiffuse>(&Vec4::new(0.8, 0.8, 0.8, 1.0))
        .write::<OccStyleMaterialSpecular>(&Vec3::new(1.0, 1.0, 1.0))
        .write::<OccStyleMaterialShininess>(&200.)
        .write::<OccStyleMaterialEmissive>(&Vec3::zero())
        .write::<OccStyleMaterialTransparent>(&false);
    texture.write::<OccStyleMaterialDiffuseTex>(w)
});

let mut effect_writer = global_entity_of::<OccStyleEffectControlEntity>().entity_writer();
let effect = effect_writer
    .new_entity(|w| w.write::<OccStyleEffectShadeType>(&OccStyleEffectType::Lighted));
occ_writer.write::<OccStyleMaterialEffect>(occ_material, effect.some_handle());
```

## Scene model wiring

### Standard pattern (covers 90% of cases)

```rust
let child = writer.create_root_child();
writer.set_local_matrix(child, Mat4::translate((x, y, z)).into_f64());
writer.create_scene_model(material, mesh, child);
```

Internally: creates `StandardModelEntity` (mesh + material) and `SceneModelEntity` (model → node → scene).

### Non-standard model types (manual wiring)

```rust
let child = writer.create_root_child();
let scene = writer.expect_target_scene().some_handle();

let std_model = writer.std_model_writer.new_entity(|w| {
    w.write::<StandardModelRefAttributesMeshEntity>(&mesh.some_handle())
        .write::<StdModelOccStyleMaterialPayload>(&occ_material.some_handle())
});

writer.model_writer.new_entity(|w| {
    w.write::<SceneModelStdModelRenderPayload>(&std_model.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&child.some_handle())
});
```

## Lights

### Directional light

```rust
let node = writer.create_root_child();
writer.set_local_matrix(node, Mat4::lookat(Vec3::splat(100.), Vec3::splat(0.), UP).into_f64());
DirectionalLightDataView {
    illuminance: Vec3::splat(5.),
    node,
    scene: writer.expect_target_scene(),
}
.write(&mut writer.directional_light_writer);
```

### Point light

```rust
let node = writer.create_root_child();
writer.set_local_matrix(node, Mat4::translate((5., 10., 2.)).into_f64());
PointLightDataView {
    intensity: Vec3::new(1., 1., 1.) * 100.,  // candela
    cutoff_distance: 40.,
    node,
    scene: writer.expect_target_scene(),
}
.write(&mut writer.point_light_writer);
```

### Spot light

```rust
let node = writer.create_root_child();
writer.set_local_matrix(node, Mat4::lookat(from, to, up).into_f64());
SpotLightDataView {
    intensity: Vec3::new(1., 0., 0.) * 1800.,
    cutoff_distance: 10.,
    half_cone_angle: Deg::by(30.).to_rad(),
    half_penumbra_angle: Deg::by(25.).to_rad(),
    node,
    scene: writer.expect_target_scene(),
}
.write(&mut writer.spot_light_writer);
```

## Transforms

All use `Mat4<f64>` (f64 precision). Convert from f32 with `.into_f64()`.

```rust
// Translation
writer.set_local_matrix(node, Mat4::translate((1.0, 0.0, -2.0)).into_f64());

// Translation + scale
writer.set_local_matrix(node, Mat4::translate((2., 0., 3.)) * Mat4::scale((2., 1., 1.)));

// LookAt (for lights or oriented objects)
writer.set_local_matrix(node, Mat4::lookat(from, to, up).into_f64());
```

## ParametricSurface trait

Defined in `rendiation_mesh_generator`:

```rust
pub trait ParametricSurface {
    /// Map [0,1]² UV to a 3D point on the surface.
    fn position(&self, position: Vec2<f32>) -> Vec3<f32>;

    /// Surface normal (not guaranteed normalized). Default uses finite differences.
    fn normal_dir(&self, position: Vec2<f32>) -> Vec3<f32> { /* finite diff */ }
}
```

Built-in surfaces: `ParametricPlane`, `UVSphere`, `RotateSweep<T>`, `FixedSweepSurface<T,P>`, `Transformed3D<T>`, `ParametricSurfaceRangeMapping<T>`.

Custom implementations like `NurbsSurface<f32>` and `RationalBezierSurface<f32>` live in `rendiation_parametric_rendering`.

## Test content module pattern

1. Create `application/viewer/src/viewer/test_content/your_test.rs`
2. Define a `pub fn load_xxx_test(writer: &mut SceneWriter)` (or with additional params)
3. Register in `test_content/mod.rs`:
   ```rust
   mod your_test;
   pub use your_test::*;
   ```
4. Call from `default_scene.rs`:
   ```rust
   load_xxx_test(writer);
   ```

## Build pipeline example

Full end-to-end pattern:

```rust
use rendiation_algebra::*;
use rendiation_mesh_generator::*;
use crate::*;

pub fn load_my_geometry_test(writer: &mut SceneWriter) {
    // 1. Define or obtain a parametric surface
    let surface = /* impl ParametricSurface */;

    // 2. Triangulate into a mesh
    let mesh = build_attributes_mesh(|builder| {
        builder.triangulate_parametric(&surface, TessellationConfig { u: 32, v: 32 }, true);
    })
    .build();

    // 3. Write mesh to scene
    let mesh = writer.write_solid_attribute_mesh(mesh).mesh;

    // 4. Create material
    let material = PhysicalSpecularGlossinessMaterialDataView {
        albedo: Vec3::new(0.7, 0.7, 0.8),
        ..Default::default()
    }
    .write(&mut writer.pbr_sg_mat_writer);
    let material = SceneMaterialDataView::PbrSGMaterial(material);

    // 5. Create node, set transform, wire together
    let child = writer.create_root_child();
    writer.set_local_matrix(child, Mat4::translate((0., 0., 0.)).into_f64());
    writer.create_scene_model(material, mesh, child);
}
```

## View-dependent transform

For models that need view-dependent behavior (always face camera, fixed screen size):

```rust
writer.model_writer.write::<SceneModelViewDependentTransformOcc>(
    model_handle,
    Some(OccStyleViewDepConfig {
        transform_ty: OccStyleTransform::Dimension3 {
            anchor_point: Vec3::new(0., 0., 0.),
        },
        mode: OccStyleMode::NotZoomRotate,
    }),
);
```

## Wide Line Rendering

Wide lines render screen space width anti-aliased line segments in 3D. Each segment is defined by start/end world-space points with per-vertex colors.

### WideLineVertex format

```rust
// Definition in extension/wide-line/src/lib.rs
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WideLineVertex {
    pub start: Vec3<f32>,  // segment start in world space
    pub end:   Vec3<f32>,  // segment end in world space
    pub color: Vec4<f32>,  // per-vertex rgba
}
```

The final fragment color is `per_vertex_color * WideLineColor` where `WideLineColor` is the global multiplier on the model entity (defaults to white).

### WideLineModelEntity components

| Component | Type | Default | Purpose |
|-----------|------|---------|---------|
| `WideLineWidth` | `f32` | 1.0 | Line width in screen pixels |
| `WideLineColor` | `Vec4<f32>` | (1,1,1,1) | Global color multiplier |
| `WideLineStylePattern` | `u32` | 0 | Bit pattern for dashed line (0 = solid) |
| `WideLineStyleFactor` | `f32` | 1.0 | Dash repetition scale |
| `WideLineEnableRoundJoint` | `bool` | false | Rounded segment joints |
| `WideLineMeshBuffer` | `ExternalRefPtr<Vec<u8>>` | — | Byte buffer of `WideLineVertex` array |

For curves or procedural geometry, build `Vec<WideLineVertex>` directly:

### Scene wiring

Wide lines use `SceneModelWideLineRenderPayload` instead of `StandardModel`:

```rust
let wide_line_model = global_entity_of::<WideLineModelEntity>()
    .entity_writer()
    .new_entity(|w| {
        w.write::<WideLineWidth>(&3.0)
          .write::<WideLineStylePattern>(&0xFFC0)   // dashed
          .write::<WideLineStyleFactor>(&6.0)
          .write::<WideLineMeshBuffer>(&mesh_buffer)
          // WideLineColor defaults to white, omitted
    });

let child = writer.create_root_child();
writer.set_local_matrix(child, Mat4::translate((x, y, z)).into_f64());

let scene = writer.expect_target_scene().some_handle();
writer.model_writer.new_entity(|w| {
    w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
      .write::<SceneModelBelongsToScene>(&scene)
      .write::<SceneModelRefNode>(&child.some_handle())
});
```

### Line style examples

| Pattern | Description |
|---------|-------------|
| `0` | Solid line |
| `0xFFC0` | Long dash (bits 15..6 set) |
| `0x0F0F` | Regular dash (alternating 4 on, 4 off) |
| `0xFF18` | Dash-dot pattern |
| `0x3333` | Dense dot pattern |
=
