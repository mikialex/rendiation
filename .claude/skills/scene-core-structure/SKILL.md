---
name: scene-core-structure
description: >
  Reference for the scene data model in rendiation (scene/core). Covers all entity types
  (SceneEntity, SceneNodeEntity, SceneModelEntity, StandardModelEntity, camera, lights,
  mesh, material, animation, skin), their component types, foreign-key relationships,
  the scene graph node hierarchy, transform propagation, SceneWriter API, and the
  StandardModel rendering pattern. Depends on database-schema for the underlying relational database layer.
  Use when understanding the scene model, adding new scene entity types, or working with
  SceneWriter/SceneReader.
metadata:
  version: "1.0"
  updated: "2026-05-17"
  depends: database-schema
---

The `scene/core` crate defines the scene data model on top of the `database` relational database layer (see [[database-schema]]). Key files:

| File | Purpose |
|------|---------|
| [scene/core/src/lib.rs](scene/core/src/lib.rs) | Crate root, `SceneEntity`, `register_scene_core_data_model()` |
| [scene/core/src/node.rs](scene/core/src/node.rs) | Scene graph nodes and world-transform derivation |
| [scene/core/src/model.rs](scene/core/src/model.rs) | `SceneModelEntity`, `StandardModelEntity`, model-to-scene wiring |
| [scene/core/src/mesh.rs](scene/core/src/mesh.rs) | `AttributesMeshEntity`, vertex buffer relations, instanced meshes |
| [scene/core/src/material.rs](scene/core/src/material.rs) | Unlit, PBR specular-glossiness, PBR metallic-roughness materials |
| [scene/core/src/camera.rs](scene/core/src/camera.rs) | `SceneCameraEntity` with perspective/orthographic/custom projection |
| [scene/core/src/light.rs](scene/core/src/light.rs) | Point, spot, and directional lights |
| [scene/core/src/texture.rs](scene/core/src/texture.rs) | 2D textures, cube maps, samplers |
| [scene/core/src/buffer.rs](scene/core/src/buffer.rs) | Raw GPU buffer storage |
| [scene/core/src/animation.rs](scene/core/src/animation.rs) | Animation assets and channels |
| [scene/core/src/skin.rs](scene/core/src/skin.rs) | Skinning and joints |
| [scene/core/src/writer.rs](scene/core/src/writer.rs) | `SceneWriter` — unified write interface |
| [scene/core/src/reader.rs](scene/core/src/reader.rs) | `SceneReader` — unified read interface |

All entity/component declarations are in the crate root's `register_scene_core_data_model()`.

## Entity relationship overview

```
SceneEntity
 ├─ SceneHDRxEnvBackgroundCubeMap ──────────→ SceneTextureCubeEntity
 │
 ├─ [light refs] ──→ Point/Spot/DirectionalLightEntity
 ├─ [model refs] ──→ SceneModelEntity
 ├─ [camera refs] ─→ SceneCameraEntity
 └─ [animation refs] → SceneAnimationEntity

SceneNodeEntity
 ├─ SceneNodeParentIdx ─────────────────────→ SceneNodeEntity (optional parent)
 │
 ├─ [model refs] ──→ SceneModelEntity (via SceneModelRefNode)
 ├─ [camera refs] ─→ SceneCameraEntity (via SceneCameraNode)
 ├─ [light refs] ──→ Point/Spot/DirectionalLightEntity (via *RefNode)
 ├─ [skin refs] ───→ SceneSkinEntity (via SceneSkinRoot)
 └─ [joint refs] ──→ SceneJointEntity (via SceneJointRefNode)

SceneModelEntity
 ├─ SceneModelBelongsToScene ───────────────→ SceneEntity
 ├─ SceneModelRefNode ──────────────────────→ SceneNodeEntity
 └─ SceneModelStdModelRenderPayload ────────→ StandardModelEntity

StandardModelEntity
 ├─ StandardModelRefAttributesMeshEntity ───→ AttributesMeshEntity
 ├─ StandardModelRef{Unlit,PbrSG,PbrMR}Material → material entity
 └─ StandardModelRefSkin ───────────────────→ SceneSkinEntity (optional)
```

Key principle: **SceneEntity** is the container. **SceneNodeEntity** provides position (transform). **SceneModelEntity** bridges a **StandardModelEntity** (what to render: mesh + material) to a specific node (where to place it) in a specific scene.

## Entity types and their components

### SceneEntity — top-level scene container

```
declare_entity!(SceneEntity)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `SceneSolidBackground` | `Option<Vec3<f32>>` | Solid background color |
| `SceneGradientBackgroundInfo` | `Option<SceneGradientBackgroundParam>` | Gradient background |
| `SceneHDRxEnvBackgroundInfo` | `Option<SceneHDRxEnvBackgroundParameter>` | HDR environment (intensity + transform) |
| `SceneHDRxEnvBackgroundCubeMap` | FK → `SceneTextureCubeEntity` | HDR environment cubemap |

### SceneNodeEntity — scene graph node

```
declare_entity!(SceneNodeEntity)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `SceneNodeLocalMatrixComponent` | `Mat4<f64>` | Local transform (defaults to identity, f64 for precision) |
| `SceneNodeVisibleComponent` | `bool` | Visibility flag (defaults to true) |
| `SceneNodeParentIdx` | FK → `SceneNodeEntity` | Optional parent for scene hierarchy |

**World matrix**: Derived by `GlobalNodeDerive` — propagates `parent_world * local` from root to leaves. Net visibility propagates similarly (`visible && all_parents_visible`).

### SceneModelEntity — bridge between render payload and scene

```
declare_entity!(SceneModelEntity)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `SceneModelBelongsToScene` | FK → `SceneEntity` | Which scene this model belongs to |
| `SceneModelRefNode` | FK → `SceneNodeEntity` | Which node positions this model |
| `SceneModelStdModelRenderPayload` | FK → `StandardModelEntity` | What to render |

### StandardModelEntity — renderable payload

```
declare_entity!(StandardModelEntity)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `StandardModelRefAttributesMeshEntity` | FK → `AttributesMeshEntity` | Mesh to render |
| `StandardModelRefUnlitMaterial` | FK → `UnlitMaterialEntity` | Unlit material (exactly one material type) |
| `StandardModelRefPbrSGMaterial` | FK → `PbrSGMaterialEntity` | PBR specular-glossiness material |
| `StandardModelRefPbrMRMaterial` | FK → `PbrMRMaterialEntity` | PBR metallic-roughness material |
| `StandardModelRefSkin` | FK → `SceneSkinEntity` | Optional skinning |
| `StandardModelRasterizationOverride` | `Option<RasterizationStates>` | Optional rasterization state override |

### AttributesMeshEntity — GPU mesh data

```
declare_entity!(AttributesMeshEntity)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `AttributesMeshEntityTopology` | `MeshPrimitiveTopology` | Triangle list, line list, etc. |
| `AttributesMeshBoundingConfig` | `BoundingConfig` | Computed or user-defined bounding box |
| `AttributeIndexRef` | → `BufferEntity` (via `SceneBufferView` components) | Index buffer |

Vertex buffers are stored via a separate relation entity:

```
declare_entity!(AttributesMeshEntityVertexBufferRelation)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `AttributesMeshEntityVertexBufferSemantic` | `AttributeSemantic` | Position, normal, uv, etc. |
| `fk → AttributesMeshEntity` | FK | Which mesh this vertex buffer belongs to |
| `AttributeVertexRef` | → `BufferEntity` (via `SceneBufferView` components) | Vertex buffer data |

### Camera

```
declare_entity!(SceneCameraEntity)
```

| Component | Type | Purpose |
|-----------|------|---------|
| `SceneCameraNode` | FK → `SceneNodeEntity` | Camera transform (from node world matrix) |
| `SceneCameraPerspective` | `Option<PerspectiveProjection<f32>>` | Perspective projection |
| `SceneCameraOrthographic` | `Option<OrthographicProjection<f32>>` | Orthographic projection |
| `SceneCameraProjectionCustomOverride` | `Option<Mat4<f32>>` | Custom projection override |

### Lights

```
declare_entity!(PointLightEntity)
declare_entity!(SpotLightEntity)
declare_entity!(DirectionalLightEntity)
```

All share the pattern: FK → `SceneEntity` (belongs to scene) + FK → `SceneNodeEntity` (position/direction from node transform).

| Type | Unique components |
|------|-------------------|
| PointLight | `PointLightIntensity: Vec3<f32>` (cd), `PointLightCutOffDistance: f32` |
| SpotLight | `SpotLightIntensity: Vec3<f32>`, `SpotLightCutOffDistance: f32`, `SpotLightHalfConeAngle: f32`, `SpotLightHalfPenumbraAngle: f32` |
| DirectionalLight | `DirectionalLightIlluminance: Vec3<f32>` (lux) |

### Materials

```
declare_entity!(UnlitMaterialEntity)       // color + optional alpha texture
declare_entity!(PbrSGMaterialEntity)       // specular-glossiness PBR
declare_entity!(PbrMRMaterialEntity)       // metallic-roughness PBR
```

All material entities have texture slots that reference `SceneTexture2dEntity` + `SceneSamplerEntity` pairs via `TextureWithSamplingForeignKeys`.

| Material | Key components | Defaults |
|----------|---------------|----------|
| Unlit | `UnlitMaterialColorComponent: Vec4<f32>` | (1,1,1,1) |
| PBR SG | `PbrSGMaterialAlbedoComponent: Vec3<f32>`, `PbrSGMaterialSpecularComponent: Vec3<f32>`, `PbrSGMaterialGlossinessComponent: f32` | (1,1,1), (0,0,0), 0.5 |
| PBR MR | `PbrMRMaterialBaseColorComponent: Vec3<f32>`, `PbrMRMaterialMetallicComponent: f32`, `PbrMRMaterialRoughnessComponent: f32` | (1,1,1), 0.0, 0.5 |

All have `AlphaConfig` (mode/blend/cutoff) and `EmissiveComponent` + emissive texture.

### Other entities

| Entity | Purpose |
|--------|---------|
| `SceneTexture2dEntity` | 2D texture (direct data or URI) |
| `SceneTextureCubeEntity` | Cube map (6 FK faces → `SceneTexture2dEntity`) |
| `SceneSamplerEntity` | Texture sampler configuration |
| `BufferEntity` | Raw GPU buffer (`Arc<Vec<u8>>`) |
| `InstanceMeshInstanceEntity` | Instanced mesh (world matrix + ref to `AttributesMeshEntity`) |
| `SceneAnimationEntity` | Animation asset (FK → `SceneEntity`) |
| `SceneAnimationChannelEntity` | Animation channel (targets `SceneNodeEntity`, stores interpolation + keyframe buffers) |
| `SceneSkinEntity` | Skinning definition (root node FK) |
| `SceneJointEntity` | Joint (FK → node + skin, stores joint index + inverse bind matrix) |

## SceneWriter API

Defined in [scene/core/src/writer.rs](scene/core/src/writer.rs). Constructed via `SceneWriter::from_global(scene)`.

### Entity writers (public fields)

Each entity type has a dedicated writer field:

```rust
writer.node_writer           // TableWriter<SceneNodeEntity>
writer.std_model_writer      // TableWriter<StandardModelEntity>
writer.model_writer          // TableWriter<SceneModelEntity>
writer.mesh_writer           // AttributesMeshEntityFromAttributesMeshWriter
writer.pbr_sg_mat_writer     // TableWriter<PbrSGMaterialEntity>
writer.pbr_mr_mat_writer     // TableWriter<PbrMRMaterialEntity>
writer.unlit_mat_writer      // TableWriter<UnlitMaterialEntity>
writer.camera_writer         // TableWriter<SceneCameraEntity>
writer.directional_light_writer
writer.point_light_writer
writer.spot_light_writer
writer.scene_writer          // TableWriter<SceneEntity>
writer.tex_writer            // TableWriter<SceneTexture2dEntity>
writer.cube_writer           // TableWriter<SceneTextureCubeEntity>
writer.sampler_writer        // TableWriter<SceneSamplerEntity>
writer.buffer_writer         // TableWriter<BufferEntity>
writer.animation             // TableWriter<SceneAnimationEntity>
writer.animation_channel     // TableWriter<SceneAnimationChannelEntity>
writer.skin_writer           // TableWriter<SceneSkinEntity>
writer.joint_writer          // TableWriter<SceneJointEntity>
```

### Key methods

| Method | Purpose |
|--------|---------|
| `expect_target_scene()` → `EntityHandle<SceneEntity>` | Get the active scene (panics if none) |
| `replace_target_scene(Option<...>)` | Switch target scene temporarily |
| `create_root_child()` → `EntityHandle<SceneNodeEntity>` | Create a node with no parent |
| `create_child(parent)` → `EntityHandle<SceneNodeEntity>` | Create a node parented to `parent` |
| `set_local_matrix(node, Mat4<f64>)` | Set node's local transform |
| `get_local_mat(node)` → `Option<Mat4<f64>>` | Read node's local transform |
| `create_scene_model(material, mesh, node)` | Create StandardModel + SceneModel, wire to node |
| `write_solid_attribute_mesh(mesh)` | Write `AttributesMesh` data, return handles |
| `write_attribute_mesh(mesh)` | Write non-solid mesh |
| `set_solid_background(color: Vec3<f32>)` | Set solid background |
| `set_gradient_background(param)` | Set gradient background |
| `set_hdr_env_background(cube, intensity, transform)` | Set HDR environment |
| `texture_sample_pair_writer()` | Helper for creating texture + sampler pairs |

## StandardModel pattern

The standard path for creating a renderable object:

```
1. Create AttributesMesh (GPU mesh data)
2. Create material entity (Unlit/PbrSG/PbrMR)
3. Create SceneNodeEntity (position via transform)
4. SceneWriter::create_scene_model(material, mesh, node)
   → internally creates StandardModelEntity + SceneModelEntity
```

`create_scene_model` accepts a `SceneMaterialDataView` enum:
```rust
SceneMaterialDataView::PbrSGMaterial(handle)
SceneMaterialDataView::PbrMRMaterial(handle)
SceneMaterialDataView::UnlitMaterial(handle)
SceneMaterialDataView::Other  // sentinel
```

At read time (`SceneReader::read_std_model`), materials are resolved by priority: PBR MR first, then PBR SG, then Unlit.

## Scene graph and transforms

- **Local transform**: `Mat4<f64>` on each `SceneNodeEntity` via `SceneNodeLocalMatrixComponent`
- **World transform**: Derived by `GlobalNodeDerive` — `node_world_mat(this, parent) = parent_world * local` (or just `local` for roots)
- **Net visibility**: Derived — `node_net_visible(this, parent) = this_visible && parent_net_visible`
- **Model world matrix**: Derived by joining node world matrices with `SceneModelRefNode` reverse refs
- **Camera transform**: Node world matrix + projection → `CameraTransform` (with view, projection, VP, inverse matrices)

## Registration

All entities, components, and foreign keys for the scene model are registered in `register_scene_core_data_model()` ([scene/core/src/lib.rs](scene/core/src/lib.rs#L44)). This function must be called during application initialization before any scene data is written.
