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
  pub uniform:
    FastHashMap<RawEntityHandle, UniformBufferCachedDataView<UniformArrayWithLengthInfo<T>>>,
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
        UniformBufferCachedDataView::create(&gpu.device, uniform_array.buffer),
      );
    }
  }
}

#[derive(Default)]
pub struct PerSceneLightArray<T: Std140> {
  pub buffer: UniformArrayWithLengthInfo<T>,
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

    self.buffer.lights.set(self.buffer.length.x as usize, light);
    self.buffer.length.x += 1;

    self
      .mapping
      .insert(light_id, self.buffer.length.x as u32 - 1);
  }
}

const LIGHT_LIST_LEN: usize = 8;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct UniformArrayWithLengthInfo<T: Std140> {
  pub length: Vec4<u32>, // use vec4 for alignment, only .x is the length info
  pub lights: Shader140Array<T, LIGHT_LIST_LEN>,
}

unsafe impl<T: Std140 + Zeroable> Zeroable for UniformArrayWithLengthInfo<T> {}
unsafe impl<T: Std140 + Pod> Pod for UniformArrayWithLengthInfo<T> {}
unsafe impl<T: Std140> Std140 for UniformArrayWithLengthInfo<T> {
  const ALIGNMENT: usize = Shader140Array::<T, LIGHT_LIST_LEN>::ALIGNMENT;
}

#[derive(Clone)]
pub struct UniformArrayWithLengthInfoShaderPtr<T> {
  access: BoxedShaderPtr,
  phantom: PhantomData<T>,
}

impl<T: Std140 + ShaderSizedValueNodeType> IntoShaderIterator
  for UniformArrayWithLengthInfoShaderPtr<T>
{
  type ShaderIter = ShaderStaticArrayReadonlyIter<Shader140Array<T, 8>, T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    let lights_ptr = self.access.field_index(1);
    let lights_view = <Shader140Array<T, 8>>::create_readonly_view_from_raw_ptr(lights_ptr);

    let length = <Vec4<u32>>::create_readonly_view_from_raw_ptr(self.access.field_index(0));
    let length_clamp = length.load().x();
    ShaderStaticArrayReadonlyIter::from_array_clamp_length(lights_view, length_clamp)
  }
}

impl<T: Std140 + ShaderSizedValueNodeType> ReadonlySizedShaderPtrView
  for UniformArrayWithLengthInfoShaderPtr<T>
{
  type Node = UniformArrayWithLengthInfo<T>;

  fn load(&self) -> Node<Self::Node> {
    unsafe { self.access.load().into_node() }
  }

  fn raw(&self) -> &BoxedShaderPtr {
    &self.access
  }
}

impl<T: Std140 + ShaderSizedValueNodeType> SizedShaderPtrView
  for UniformArrayWithLengthInfoShaderPtr<T>
{
  fn store(&self, value: impl Into<Node<Self::Node>>) {
    self.access.store(value.into().handle());
  }
}

impl<T: Std140> ShaderAbstractPtrAccess for UniformArrayWithLengthInfo<T> {
  type PtrView = UniformArrayWithLengthInfoShaderPtr<T>;
  type ReadonlyPtrView = UniformArrayWithLengthInfoShaderPtr<T>;

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

impl<T: ShaderSizedValueNodeType + Std140> ShaderNodeType for UniformArrayWithLengthInfo<T> {
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(ShaderValueSingleType::Sized(Self::sized_ty()))
  }
}

impl<T: ShaderSizedValueNodeType + Std140> ShaderSizedValueNodeType
  for UniformArrayWithLengthInfo<T>
{
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::Struct(
      ShaderStructMetaInfo::new("UniformArrayWithLengthInfo")
        .add_field::<Vec4<u32>>("length")
        .add_field::<Shader140Array<T, LIGHT_LIST_LEN>>("lights"),
    )
  }

  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Struct(vec![
      ShaderStructFieldInitValue::Primitive(self.length.to_primitive()),
      ShaderStructFieldInitValue::Array(
        self
          .lights
          .inner
          .iter()
          .map(|v| v.inner.to_value())
          .collect(),
      ),
    ])
  }
}
