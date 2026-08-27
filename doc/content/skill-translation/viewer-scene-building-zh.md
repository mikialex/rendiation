---
name: viewer-scene-building
description: >
  在 rendiation viewer 应用中构建 3D 测试场景的实用配方。涵盖从
  ParametricSurface 生成网格 (build_attributes_mesh、triangulate_parametric)、
  所有材质类型的创建模式、场景模型接线、灯光设置、
  变换,以及 test-content 模块模式。
  底层场景数据模型(实体类型、组件、外键、SceneWriter API)
  参见 scene-core-structure。关系数据库层参见 database-schema。
metadata:
  version: "2.1"
  updated: "2026-05-17"
  depends: [scene-core-structure, database-schema]
---

面向 rendiation viewer 的实用场景构建配方。场景数据模型与 `SceneWriter` API 参考见 [[scene-core-structure]]。数据库层见 [[database-schema]]。

这里用到的关键文件:

| 文件 | 用途 |
|------|------|
| [application/viewer/src/viewer/default_scene.rs](../../../../../rendiation/application/viewer/src/viewer/default_scene.rs) | 规范的场景搭建模式 |
| [application/viewer/src/viewer/test_content/](../../../../../rendiation/application/viewer/src/viewer/test_content/) | 测试场景函数 |
| [application/viewer/src/viewer/test_content/widen_line.rs](../../../../../rendiation/application/viewer/src/viewer/test_content/widen_line.rs) | 宽线测试示例 |
| [content/mesh/generator/src/lib.rs](../../../../../rendiation/content/mesh/generator/src/lib.rs) | `build_attributes_mesh`、`AttributesMeshBuilder` |
| [content/mesh/generator/src/builder/mod.rs](../../../../../rendiation/content/mesh/generator/src/builder/mod.rs) | `triangulate_parametric`、`TessellationConfig` |
| [content/mesh/generator/src/parametric.rs](../../../../../rendiation/content/mesh/generator/src/parametric.rs) | `ParametricSurface` trait |

## 导入

```rust
use rendiation_algebra::*;
use rendiation_mesh_generator::*;
use crate::*;
```

## 网格生成

### 从 ParametricSurface 生成

```rust
let mesh = build_attributes_mesh(|builder| {
    builder.triangulate_parametric(
        &surface,
        TessellationConfig { u: 32, v: 32 },
        true,  // keep_grouping: 推入一个新的绘制组
    );
})
.build();
```

- `build_attributes_mesh` 创建 `AttributesMeshBuilder`,执行闭包,完成网格构建,把图元收集到 `AttributesMeshData` 中。
- `AttributesMeshData` 上的 `.build()` 生成 GPU 就绪的网格句柄。
- `TessellationConfig.u` / `.v` 控制细分密度。

### 一个网格中包含多个面

```rust
let mesh = build_attributes_mesh(|builder| {
    for face in cube.make_faces() {
        builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
    }
})
.build();
```

### 写入网格数据

```rust
let mesh = writer.write_solid_attribute_mesh(attribute_mesh).mesh;
```

对于由资产系统追踪的网格:
```rust
let mesh = writer
    .write_solid_attribute_mesh_data_uri(attribute_mesh, mesh_source)
    .mesh;
```

说明:`write_solid_attribute_mesh` 与 `write_solid_attribute_mesh_data_uri` 来自 `WriteSolidAttributeMesh` trait(`effect/plane_array_clip` crate,为 `SceneWriter` 实现),viewer 中通过 `use crate::*` 引入。两个方法都返回 `AttributesMeshEntities`,其中的 `.mesh` 字段即 `EntityHandle<AttributesMeshEntity>`。

## 材质创建模式

### PBR Specular-Glossiness(彩色物体最简单的方式)

```rust
let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::new(0.8, 0.3, 0.3),
    ..Default::default()
}
.write(&mut writer.pbr_sg_mat_writer);
let material = SceneMaterialDataView::PbrSGMaterial(material);
```

### 带 alpha 混合的 PBR Specular-Glossiness

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

### 带纹理的 PBR Metallic-Roughness

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

### OccStyle(CAD 风格)

```rust
// 完整类型路径见 extension/occ-style-material
use rendiation_occ_style_material::*;

let mut occ_writer = global_entity_of::<OccStyleMaterialEntity>().entity_writer();
let occ_material = occ_writer.new_entity(|w| {
    let w = w
        .write::<OccStyleMaterialDiffuse>(&Vec4::new(0.8, 0.8, 0.8, 1.0))
        .write::<OccStyleMaterialSpecular>(&Vec3::new(1.0, 1.0, 1.0))
        .write::<OccStyleMaterialShininess>(&200.)
        .write::<OccStyleMaterialEmissive>(&Vec3::zero());
    texture.write::<OccStyleMaterialDiffuseTex>(w)
});

let mut effect_writer = global_entity_of::<OccStyleEffectControlEntity>().entity_writer();
let effect = effect_writer
    .new_entity(|w| w.write::<OccStyleEffectShadeType>(&OccStyleEffectType::Lighted));
occ_writer.write::<OccStyleMaterialEffect>(occ_material, effect.some_handle());
```

## 场景模型接线

### 标准模式(覆盖 90% 的场景)

```rust
let child = writer.create_root_child();
writer.set_local_matrix(child, Mat4::translate((x, y, z)).into_f64());
writer.create_scene_model(material, mesh, child, scene);
```

内部:创建 `StandardModelEntity`(网格 + 材质)和 `SceneModelEntity`(模型 → 节点 → 场景)。

### 非标准模型类型(手动接线)

```rust
let child = writer.create_root_child();
let scene = scene.some_handle();

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

## 灯光

以下示例中的 `scene` 均来自测试函数的参数(见下文"Test content module pattern")。

### 平行光

```rust
let node = writer.create_root_child();
writer.set_local_matrix(node, Mat4::lookat(Vec3::splat(100.), Vec3::splat(0.), UP).into_f64());
DirectionalLightDataView {
    illuminance: Vec3::splat(5.),
    node,
    scene,
}
.write(&mut writer.directional_light_writer);
```

### 点光源

```rust
let node = writer.create_root_child();
writer.set_local_matrix(node, Mat4::translate((5., 10., 2.)).into_f64());
PointLightDataView {
    intensity: Vec3::new(1., 1., 1.) * 100.,  // 坎德拉(candela)
    cutoff_distance: 40.,
    node,
    scene,
}
.write(&mut writer.point_light_writer);
```

### 聚光灯

```rust
let node = writer.create_root_child();
writer.set_local_matrix(node, Mat4::lookat(from, to, up).into_f64());
SpotLightDataView {
    intensity: Vec3::new(1., 0., 0.) * 1800.,
    cutoff_distance: 10.,
    half_cone_angle: Deg::by(30.).to_rad(),
    half_penumbra_angle: Deg::by(25.).to_rad(),
    node,
    scene,
}
.write(&mut writer.spot_light_writer);
```

## 变换

全部使用 `Mat4<f64>`(f64 精度)。从 f32 转换用 `.into_f64()`。

```rust
// 平移
writer.set_local_matrix(node, Mat4::translate((1.0, 0.0, -2.0)).into_f64());

// 平移 + 缩放
writer.set_local_matrix(node, Mat4::translate((2., 0., 3.)) * Mat4::scale((2., 1., 1.)));

// LookAt(用于灯光或定向物体)
writer.set_local_matrix(node, Mat4::lookat(from, to, up).into_f64());
```

## ParametricSurface trait

定义于 `rendiation_mesh_generator`:

```rust
pub trait ParametricSurface {
    /// 将 [0,1]² 的 UV 映射到曲面上的 3D 点。
    fn position(&self, position: Vec2<f32>) -> Vec3<f32>;

    /// 曲面法线(不保证已归一化)。默认使用有限差分。
    fn normal_dir(&self, position: Vec2<f32>) -> Vec3<f32> { /* finite diff */ }
}
```

内置曲面:`ParametricPlane`、`UVSphere`、`RotateSweep<T>`、`FixedSweepSurface<T,P>`、`Transformed3D<T>`、`ParametricSurfaceRangeMapping<T>`。

自定义实现如 `NurbsSurface<f32>` 与 `RationalBezierSurface<f32>` 位于 `rendiation_parametric_rendering`。

## Test content 模块模式

- 创建 `application/viewer/src/viewer/test_content/your_test.rs`
- 定义 `pub fn load_xxx_test(writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>)`(可带额外参数,如纹理/网格数据源)
- 在 `test_content/mod.rs` 中注册:
  ```rust
  mod your_test;
  pub use your_test::*;
  ```
- 从 `default_scene.rs` 调用:
  ```rust
  load_xxx_test(writer, scene);
  ```

## 构建管线示例

完整的端到端模式:

```rust
use rendiation_algebra::*;
use rendiation_mesh_generator::*;
use crate::*;

pub fn load_my_geometry_test(writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
    // 定义或获取一个参数化曲面
    let surface = /* impl ParametricSurface */;

    // 三角化生成网格
    let mesh = build_attributes_mesh(|builder| {
        builder.triangulate_parametric(&surface, TessellationConfig { u: 32, v: 32 }, true);
    })
    .build();

    // 把网格写入场景
    let mesh = writer.write_solid_attribute_mesh(mesh).mesh;

    // 创建材质
    let material = PhysicalSpecularGlossinessMaterialDataView {
        albedo: Vec3::new(0.7, 0.7, 0.8),
        ..Default::default()
    }
    .write(&mut writer.pbr_sg_mat_writer);
    let material = SceneMaterialDataView::PbrSGMaterial(material);

    // 创建节点、设置变换、完成接线
    let child = writer.create_root_child();
    writer.set_local_matrix(child, Mat4::translate((0., 0., 0.)).into_f64());
    writer.create_scene_model(material, mesh, child, scene);
}
```

## 视图相关变换

对于需要视图相关行为(始终面向相机、固定屏幕尺寸)的模型:

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

## 宽线渲染

宽线在 3D 中渲染具有屏幕空间宽度、抗锯齿的线段。每段由世界空间的起点/终点定义,并带有逐顶点颜色。

### WideLineVertex 格式

```rust
// 定义见 extension/wide-line/src/lib.rs
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WideLineVertex {
    pub start: Vec3<f32>,  // 线段起点(世界空间)
    pub end:   Vec3<f32>,  // 线段终点(世界空间)
    pub color: Vec4<f32>,  // 逐顶点 rgba
}
```

最终片元颜色为 `per_vertex_color * WideLineColor`,其中 `WideLineColor` 是模型实体上的全局倍率(默认为白色)。

### WideLineModelEntity 组件

| 组件 | 类型 | 默认值 | 用途 |
|-----------|------|---------|---------|
| `WideLineWidth` | `f32` | 1.0 | 屏幕像素单位的线宽 |
| `WideLineColor` | `Vec4<f32>` | (1,1,1,1) | 全局颜色倍率 |
| `WideLineStylePattern` | `u32` | 0 | 虚线位模式(0 = 实线) |
| `WideLineStyleFactor` | `f32` | 1.0 | 虚线重复缩放 |
| `WideLineEnableRoundJoint` | `bool` | false | 圆角线段连接 |
| `WideLineMeshBuffer` | `ExternalRefPtr<Vec<u8>>` | — | `WideLineVertex` 数组的字节缓冲区 |

对于曲线或程序化几何,直接构建 `Vec<WideLineVertex>`:

### 场景接线

宽线使用 `SceneModelWideLineRenderPayload` 而非 `StandardModel`:

```rust
let wide_line_model = global_entity_of::<WideLineModelEntity>()
    .entity_writer()
    .new_entity(|w| {
        w.write::<WideLineWidth>(&3.0)
          .write::<WideLineStylePattern>(&0xFFC0)   // 虚线
          .write::<WideLineStyleFactor>(&6.0)
          .write::<WideLineMeshBuffer>(&mesh_buffer)
          // WideLineColor 默认为白色,省略不写
    });

let child = writer.create_root_child();
writer.set_local_matrix(child, Mat4::translate((x, y, z)).into_f64());

let scene = scene.some_handle();
writer.model_writer.new_entity(|w| {
    w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
      .write::<SceneModelBelongsToScene>(&scene)
      .write::<SceneModelRefNode>(&child.some_handle())
});
```

### 线型示例

| 模式 | 说明 |
|---------|-------------|
| `0` | 实线 |
| `0xFFC0` | 长虚线(位 15..6 置位) |
| `0x0F0F` | 常规虚线(交替 4 开 4 关) |
| `0xFF18` | 点划线图案 |
| `0x3333` | 密集点状图案 |
