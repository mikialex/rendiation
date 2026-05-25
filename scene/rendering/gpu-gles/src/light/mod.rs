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

  output
}

pub fn sync_per_scene_uniforms<T: Std140>(
  new_data: &PerSceneLightUniformArray<T>,
  uniform_array_caches: &Arc<
    RwLock<FastHashMap<RawEntityHandle, UniformBufferDataView<UniformArrayWithLengthInfo<T>>>>,
  >,
  gpu: &GPU,
) {
  let mut uniform_array_caches__ = uniform_array_caches.write();
  let uniform_array_caches_ = &mut *uniform_array_caches__;

  for (scene_id, uniform_array) in &new_data.lists {
    if let Some(existing) = uniform_array_caches_.get(scene_id) {
      existing.write_at(&gpu.queue, &uniform_array.buffer, 0);
    } else {
      uniform_array_caches_.insert(
        *scene_id,
        UniformBufferDataView::create(&gpu.device, uniform_array.buffer),
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

#[derive(Default, Clone, Copy, Debug)]
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
