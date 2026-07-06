mod directional;
use std::marker::PhantomData;

pub use directional::*;
mod point;
use fast_hash_collection::FastHashMap;
pub use point::*;
mod spot;
pub use spot::*;

use crate::*;

#[derive(Default)]
pub struct PerSceneLightUniformArray<T: Std140> {
  pub lists: FastHashMap<RawEntityHandle, PerSceneLightArray<T>>,
}

/// if possible, light_ref_scene should iter from most important light as the light can be discard due to array length limit
pub fn compute_light_list<T: Std140 + Default>(
  light_ref_scene: impl Iterator<Item = (RawEntityHandle, RawEntityHandle, T)>,
) -> PerSceneLightUniformArray<T> {
  let mut output = PerSceneLightUniformArray::default();
  for (light, scene, light_data) in light_ref_scene {
    let list = output.lists.entry(scene).or_default();
    list.push(light, light_data);
  }

  // make sure for scene that has not light, we still get a empty array uniform
  // do this to reduce the shader variation, and make sure the empty case is correctly synced
  let scenes = get_db_set_view::<SceneEntity>();
  for (scene, _) in scenes.iter_key_value() {
    if !output.lists.contains_key(&scene) {
      output.lists.insert(scene, PerSceneLightArray::default());
    }
  }

  output
}

pub struct LightUniformInfo<T: Std140> {
  /// scene id -> per scene uniform array
  pub uniform: FastHashMap<
    RawEntityHandle,
    UniformBufferCachedDataView<UniformArrayWithLengthInfo<T, LIGHT_LIST_LEN>>,
  >,
  /// scene id -> light id -> allocation index
  pub allocation_info: FastHashMap<RawEntityHandle, FastHashMap<RawEntityHandle, u32>>,
  pub label: String,
}

pub type SharedLightUniformInfo<T> = Arc<RwLock<LightUniformInfo<T>>>;

pub fn use_shared_light_uniform_info<T: Std140>(
  cx: &mut QueryGPUHookCx,
  label: &str,
) -> SharedLightUniformInfo<T> {
  cx.use_sharable_plain_state(|| LightUniformInfo {
    uniform: Default::default(),
    allocation_info: Default::default(),
    label: label.to_string(),
  })
}

pub fn sync_per_scene_uniforms<T: Std140 + PartialEq>(
  new_data: &PerSceneLightUniformArray<T>,
  uniform_array_caches: &SharedLightUniformInfo<T>,
  gpu: &GPU,
  label: &str,
) {
  let mut uniform_array_caches__ = uniform_array_caches.write();
  let uniform_array_caches_ = &mut *uniform_array_caches__;

  uniform_array_caches_.allocation_info = new_data
    .lists
    .iter()
    .map(|(k, v)| (*k, v.mapping.clone()))
    .collect();

  let gpu_uniforms = &mut uniform_array_caches_.uniform;
  for (scene_id, uniform_array) in &new_data.lists {
    if let Some(existing) = gpu_uniforms.get(scene_id) {
      existing.set(uniform_array.buffer);
      existing.upload_with_diff(&gpu.queue);
    } else {
      gpu_uniforms.insert(
        *scene_id,
        UniformBufferCachedDataView::create(&gpu.device, uniform_array.buffer, label),
      );
    }
  }
}

#[derive(Default)]
pub struct PerSceneLightArray<T: Std140> {
  pub buffer: UniformArrayWithLengthInfo<T, LIGHT_LIST_LEN>,
  // map light id to it's allocate index in array
  pub mapping: FastHashMap<RawEntityHandle, u32>,
}

impl<T: Std140> PerSceneLightArray<T> {
  pub fn push(&mut self, light_id: RawEntityHandle, light: T) {
    if self.buffer.length.x as usize == LIGHT_LIST_LEN {
      log::warn!(
        "light list is full, light {} will not be rendered",
        light_id
      );
      return;
    }

    self.buffer.array.set(self.buffer.length.x as usize, light);
    self.buffer.length.x += 1;

    self
      .mapping
      .insert(light_id, self.buffer.length.x as u32 - 1);
  }
}

pub const LIGHT_LIST_LEN: usize = 8;

/// this util should move to upstream if others want to use
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct UniformArrayWithLengthInfo<T: Std140, const N: usize> {
  pub length: Vec4<u32>, // use vec4 for alignment, only .x is the length info
  pub array: Shader140Array<T, N>,
}

unsafe impl<T: Std140 + Zeroable, const N: usize> Zeroable for UniformArrayWithLengthInfo<T, N> {}
unsafe impl<T: Std140 + Pod, const N: usize> Pod for UniformArrayWithLengthInfo<T, N> {}
unsafe impl<T: Std140, const N: usize> Std140 for UniformArrayWithLengthInfo<T, N> {
  const ALIGNMENT: usize = Shader140Array::<T, N>::ALIGNMENT;
}

#[derive(Clone)]
pub struct UniformArrayWithLengthInfoShaderPtr<T, const N: usize> {
  access: BoxedShaderPtr,
  phantom: PhantomData<T>,
}

impl<T: Std140 + ShaderSizedValueNodeType, const N: usize> IntoShaderIterator
  for UniformArrayWithLengthInfoShaderPtr<T, N>
{
  type ShaderIter = ShaderStaticArrayReadonlyIter<Shader140Array<T, N>, T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    let array_ptr = self.access.field_index(1);
    let array_view = <Shader140Array<T, N>>::create_readonly_view_from_raw_ptr(array_ptr);

    let length = <Vec4<u32>>::create_readonly_view_from_raw_ptr(self.access.field_index(0));
    let length_clamp = length.load().x();
    ShaderStaticArrayReadonlyIter::from_array_clamp_length(array_view, length_clamp)
  }
}

impl<T, const N: usize> ReadonlySizedShaderPtrView for UniformArrayWithLengthInfoShaderPtr<T, N>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  type Node = UniformArrayWithLengthInfo<T, N>;

  fn load(&self) -> Node<Self::Node> {
    unsafe { self.access.load().into_node() }
  }

  fn raw(&self) -> &BoxedShaderPtr {
    &self.access
  }
}

impl<T, const N: usize> SizedShaderPtrView for UniformArrayWithLengthInfoShaderPtr<T, N>
where
  T: Std140 + ShaderSizedValueNodeType,
{
  fn store(&self, value: impl Into<Node<Self::Node>>) {
    self.access.store(value.into().handle());
  }
}

impl<T: Std140, const N: usize> ShaderAbstractPtrAccess for UniformArrayWithLengthInfo<T, N> {
  type PtrView = UniformArrayWithLengthInfoShaderPtr<T, N>;
  type ReadonlyPtrView = UniformArrayWithLengthInfoShaderPtr<T, N>;

  fn create_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::PtrView {
    UniformArrayWithLengthInfoShaderPtr {
      access: ptr,
      phantom: PhantomData,
    }
  }

  fn create_readonly_view_from_raw_ptr(ptr: BoxedShaderPtr) -> Self::ReadonlyPtrView {
    UniformArrayWithLengthInfoShaderPtr {
      access: ptr,
      phantom: PhantomData,
    }
  }
}

impl<T, const N: usize> ShaderNodeType for UniformArrayWithLengthInfo<T, N>
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(ShaderValueSingleType::Sized(Self::sized_ty()))
  }
}

impl<T, const N: usize> ShaderSizedValueNodeType for UniformArrayWithLengthInfo<T, N>
where
  T: ShaderSizedValueNodeType + Std140,
{
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::Struct(
      ShaderStructMetaInfo::new("UniformArrayWithLengthInfo")
        .add_field::<Vec4<u32>>("length")
        .add_field::<Shader140Array<T, N>>("array"),
    )
  }

  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Struct(vec![
      ShaderStructFieldInitValue::Primitive(self.length.to_primitive()),
      ShaderStructFieldInitValue::Array(
        self
          .array
          .inner
          .iter()
          .map(|v| v.inner.to_value())
          .collect(),
      ),
    ])
  }
}
