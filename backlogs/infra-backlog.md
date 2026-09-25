# Infra Backlog

Source: the full review of `platform/graphics/webgpu` on 2026-09-25, plus issues found while fixing it. Only **unresolved** issues are listed here; anything already fixed has been removed.

Code locations are given as "file path + symbol name", not line numbers.

---

## 1. Correctness risks

### 1.1 Buffer resize is submitted immediately on a separate encoder, with no ordering against the encoder currently being recorded

- Location:
  - `platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/gpu_raw.rs`: `ResizableGPUBuffer::resize_with_relocations`
  - `scene/rendering/batch-extractor/src/list_pool.rs`: `apply_pool_update` (ignores the `encoder` parameter it receives, creates its own encoder and submits it immediately)
- Problem: the resize/relocation copies always execute before the outer, not-yet-submitted encoder. If the outer encoder has already recorded GPU commands (e.g. compute) that write into the old buffer, those writes never reach the new buffer and the data is lost.
- Suggestion: make resize/relocation record into the encoder passed in by the caller. This changes the `ResizableLinearStorage` family of trait interfaces, so the change is fairly wide and should be scheduled separately.

### 1.2 `list_pool` writes to the underlying buffer at absolute offsets

- Location: `scene/rendering/batch-extractor/src/list_pool.rs`: the `queue.write_buffer` call for `move_writes` in `apply_pool_update`
- Problem: it takes the underlying `gpu::Buffer` via `get_gpu_buffer_view()` and writes at an absolute offset, ignoring the view offset. If `pool_buffer` is a sub-buffer of a combined buffer, the data lands in the wrong place.
- Suggestion: use `AbstractBuffer::write`, which writes at an offset relative to the view.

### 1.3 SBT allocation leaks when it only partly succeeds

- Location: `shader/ray-tracing/src/backend/wavefront_compute/sbt.rs`: `ShaderBindingTableDeviceInfo::allocate`
- Problem: the hit, miss and gen ranges are allocated one after another, and any failure returns `None` straight away via `?`. The ranges that were already allocated are never freed, and `offset_map` has no record of them.
- Suggestion: when one range fails, roll back (free) the ranges already allocated.

### 1.4 The range allocator can only panic when resize fails

- Location: `platform/graphics/webgpu/src/resource/buffer/allocator/range.rs`: `GPURangeAllocateMaintainer::apply_resize_and_relocations`
- Problem: by the time this runs, `GrowableRangeAllocator` has already committed the new allocation and the data movements. If the GPU buffer resize then fails, the two sides are out of sync, so for now the only option is `assert!`.
- Suggestion: to fail gracefully, `GrowableRangeAllocator` needs rollback support. Alternatively, check before allocating that the target size is within device limits.

### 1.5 Zero-length buffer behavior is undefined

- Location:
  - `platform/graphics/webgpu/src/resource/buffer/storage.rs`: `From<ZeroedArrayByArrayLength> for StorageBufferInit`
  - `DefaultStorageAllocator::allocate_dyn_ty` and `create_storage_buffer_range_allocate_pool` (when the initial count is 0)
- Problem: `NonZeroU64::new(0).unwrap()` panics. midc-downgrade had a zero-capacity test (`test_downgrade_list_pool_zero_capacity`) that had always been failing; it has been deleted.
- Suggestion: decide the semantics of an empty buffer first: either allocate a minimum-size buffer, or require callers to guarantee the size is never 0. Then add a test for it.

### 1.6 `TryFrom` for `GPUTypedTexture` performs no checks

- Location: `platform/graphics/webgpu/src/resource/texture/mod.rs`: `impl TryFrom<GPUTexture> for GPUTypedTexture<D, F>`
- Problem: the check code is commented out, so any texture converts to a typed texture of any dimension and format. The view-level check has been fixed; the texture level has not.
- Suggestion: validate texture dimension and format, following the view-level check.

---

## 2. Design issues

### 2.1 The device requests every adapter feature and limit, and enables experimental features

- Location: `platform/graphics/webgpu/src/lib.rs`: where `GPU::new` builds the `DeviceDescriptor`
- Problem: `required_features` and `required_limits` are set to everything the adapter supports, and `experimental_features` is enabled. `minimal_required_*` then only acts as a check, and code can silently come to depend on features that only some machines have, so it breaks on another machine.
- Suggestion: this needs an owner decision. One option is to request only `minimal_required_*` plus explicitly declared optional features.

### 2.2 The acceleration structure layout hardcodes `vertex_return: true`

- Location: `platform/graphics/webgpu/src/pipeline/mod.rs`: the `AccelerationStructure` branch of `map_shader_value_ty_to_binding_layout_type`
- Problem: this requires the experimental ray hit vertex return feature. On devices without it, layout creation fails.
- Suggestion: choose the value based on whether the feature is available, or pass it in from the shader-side descriptor.

### 2.3 The pipeline cache keys on a 64-bit hash only, with no equality check

- Location: `platform/graphics/webgpu/src/device.rs`: `get_or_cache_create_render_pipeline`, `get_or_cache_create_compute_pipeline`
- Problem: `PipelineHasher` is a 64-bit FxHash, and there is no full key to compare against. The chance of a real collision is negligible. The more realistic risk is a component forgetting to hash a field that affects the pipeline; the earlier `reversed_depth` and `BindingController.target` bugs were exactly this and have been fixed. Both kinds of error fail silently.
- Suggestion: add an opt-in debug validation mode. On a cache hit, rebuild the shader from the current component and compare the output (WGSL or naga module, plus pipeline state) with what was recorded when the pipeline was cached. On a mismatch, panic and print the component type info. This catches both collisions and missing hash fields. Keep it off by default; if it is too slow, validate only a sample of hits.
- Note: the BindGroup and BindGroupLayout caches already compare full keys; on the BindGroup side this is controlled by `BINDGROUP_CACHE_FULL_KEY_COMPARE`.

### 2.4 The two resource id namespaces overlap

- Location: `platform/graphics/webgpu/src/resource/array.rs`: `BindingResourceArray::new` uses `get_new_resource_guid()`, while regular views use `create_resource_view_guid()`
- Problem: both counters start at 0, so a binding array's pseudo view id can equal a real view's id. Cache keys are now compared in full, so a wrong bindgroup is never returned. But the view-to-bindgroup reverse index gets crossed: when one side is dropped, it also evicts the other side's bindgroups, causing unnecessary cache invalidation.
- Suggestion: generate the pseudo view id with `create_resource_view_guid()` as well.

### 2.5 Deferred destruction on wasm uses a global counter

- Location: `platform/graphics/webgpu/src/resource/defer_explicit_destroy.rs`: `DeferExplicitDestroy`
- Problem:
  - The counter tracks every encoder currently recording. As long as one long-lived encoder exists, no resource is ever destroyed.
  - `ResourceExplicitDestroy::drop` reads the counter and then pushes the resource onto the pending-destroy list; there is a race between those two steps, so destruction can be delayed until the next time the counter reaches zero.
- Suggestion: track per command buffer which pending-destroy resources it references, or at least guard the counter read and the push with a single lock.

### 2.6 Shader build failure during pipeline creation panics

- Location: `platform/graphics/webgpu/src/rendering.rs`: `RenderComponent::render`; `platform/graphics/webgpu/src/device.rs`: `get_or_cache_create_compute_pipeline_by`
- Problem: the results of `build_self(...)`, `build_pipeline_by_shader_api(...)` and `create_compute_pipeline(...)` are all `unwrap`ped, with no error return path.
- Suggestion: at minimum, print the error together with the component type info; then consider a recoverable path.

---

## 3. Performance issues

### 3.1 The hot path takes global write locks every time

- Location:
  - `GPUDevice::get_or_cache_create_render_pipeline`: takes the write lock on every draw, even on a cache hit, and runs `creator` (i.e. compiles the shader) while holding it
  - `BindingBuilder::setup_binding`: takes the bindgroup cache write lock for every group on every draw
- Status: everything is single-threaded today, so this is low priority and deferred for now.
- Suggestion: look up under a read lock first; on a miss, take the write lock and look up again. Compile outside the lock.

### 3.2 `CacheAbleBindingBuildSource` is built eagerly on every bind

- Location: `platform/graphics/webgpu/src/binding/bind_source.rs`, and `get_binding_build_source` for typed texture arrays in `resource/array.rs`
- Problem: even when the bindgroup cache hits, every bind clones an Arc, and typed texture arrays also allocate a new Vec each time.
- Suggestion: record only the view ids up front, and build `BindingResourceOwned` only on a cache miss.

### 3.3 `min_binding_size` is always `None`

- Location: `platform/graphics/webgpu/src/pipeline/mod.rs`: `map_shader_value_ty_to_binding_layout_type`
- Problem: wgpu therefore validates binding sizes late, on every draw, and errors are reported later too.
- Suggestion: for sized types, compute the minimum size when creating the layout and fill it in.

### 3.4 With statistics enabled, new GPU objects are created per pass, per frame

- Location: `platform/graphics/webgpu/src/query/`, `frame/statistics.rs`
- Problem: each pass creates a new `QuerySet` every frame, plus a resolve buffer and a staging buffer. The cost only applies when statistics are enabled.
- Suggestion: pool and reuse the query sets and buffers.

### 3.5 `AtomicImageDowngrade::clear` creates a new uniform on every call

- Location: `platform/graphics/webgpu/src/atomic_image_downgrade.rs`: `AtomicImageDowngrade::clear`
- Problem: a new uniform per call means a new bindgroup per call. It cannot simply share one uniform, though: `queue.write_buffer` calls all take effect before submit, so multiple clears in the same encoder would all read the same parameters.
- Suggestion: pass the parameters through a dynamic offset array (`UniformBufferDynamicOffsetArray`) or immediates.

---

## 4. Minor issues

- **Unclear errors when the bindgroup count does not match the pipeline**: `BindingBuilder::setup_binding` skips trailing empty groups. So when the pipeline expects more groups than were bound, the layout check does not cover them and wgpu only reports an error at draw time. Conversely, binding more groups than the pipeline has makes `layouts[group_index]` panic with an index out of bounds, with no readable message.
- **The clear dispatch can exceed the limit**: `AtomicImageDowngrade::clear` dispatches `count.div_ceil(256)` workgroups in x, which can exceed `max_compute_workgroups_per_dimension` (65535) for large images or many layers.
- **Bytes per row is wrong for compressed formats**: `WebGPU2DTextureSource::bytes_per_row` treats `block_copy_size` as bytes per pixel, but for compressed formats it is bytes per block, so the computed bytes per row is wrong.
- **Full-screen quad restores the viewport incorrectly**: `QuadDraw::render` resets the viewport to the full pass size when it finishes, overwriting any custom viewport the caller had set.
- **`VecWithStorageBuffer` bounds and error handling**: `set_value` panics on an out-of-range index (there is already a todo in the code); `removes` ignores the return value of the underlying `removes`.
- **Two `StatisticStore` issues**:
  - `iter_history_from_oldest_latest` actually iterates from newest to oldest, the opposite of its name, and skips the record at index 0.
  - `history_average` only exists for f32, while timing statistics use `StatisticStore<f64>`, so it cannot be used for them.
- **`basic_texture_usages()` may produce invalid usages (to confirm)**: it includes `STORAGE_BINDING`, but many formats (e.g. sRGB formats) do not support storage binding, so creating a texture with these usages triggers a validation error. Need to check which formats the callers use.

---

## 5. Follow-ups

- **No convenience container for storage buffers with dynamic offsets**: only `UniformBufferDynamicOffsetArray` exists. A storage buffer can be used by wrapping a sub-range view with an explicit size in `DynamicOffsetBinding`, but there is no matching array container.
- **The stricter dimension check needs validating in real scenes**: `DimensionDynamicViewCheck` used to always pass and is now strict. A static review of callers using Cube / 2DArray / CubeArray types found that they all set a matching `dimension` explicitly. Still, run the viewer and the shadow, IBL and texture pool scenes to confirm no caller relied on the old permissive check. The most likely breakage: a single-layer texture using its default view and then being converted to a `*2DArray*` type.
- **Fixes that need tests**: none of this round's fixes came with tests. Cover these first:
  - Texture dimension check
  - Slab allocator `deallocate_back` and `used_count`
  - `VecWithStorageBuffer::set_value_sub_bytes`
  - Offset mapping correctness after range allocator relocation
  - `view_byte_size`, `write` and indirect dispatch on sub-range views
  - BindGroup cache full-key comparison, and correct cache cleanup after a view is dropped
  - Sampler view cache
