use crate::*;

// todo, make this upstream
/// Currently, the naga only support struct in bind array of storage buffer, so we have to wrap the
/// array into the struct, so we have to impl some trait by hand
#[repr(transparent)]
pub struct BindlessStorageWorkaround<T> {
  array: [T],
}

impl<T: ShaderSizedValueNodeType> BindlessStorageWorkaround<T> {
  pub fn cast_slice(array: &[T]) -> &Self {
    // oh my god, i don't know if it's safe at all?
    unsafe { std::mem::transmute(array) }
  }

  pub fn read_index_shader(
    node: Node<ShaderReadOnlyStoragePtr<BindlessStorageWorkaround<T>>>,
    index: Node<u32>,
  ) -> Node<ShaderReadOnlyStoragePtr<T>> {
    let array_ptr: Node<ShaderReadOnlyStoragePtr<[T]>> = ShaderNodeExpr::FieldGet {
      field_index: 0,
      struct_node: node.handle(),
    }
    .insert_api(); // todo, this should be unsafe
    array_ptr.index(index)
  }
}

impl<T: ShaderSizedValueNodeType> ShaderNodeType for BindlessStorageWorkaround<T> {
  const TYPE: ShaderValueType =
    ShaderValueType::Single(ShaderValueSingleType::Sized(T::MEMBER_TYPE));
}

pub trait BindlessStorageWorkaroundNameHack {
  const NAME: &'static str;
}

impl BindlessStorageWorkaroundNameHack for Vec2<f32> {
  const NAME: &'static str = "BindlessStorageWorkaroundVec2f32";
}
impl BindlessStorageWorkaroundNameHack for Vec4<f32> {
  const NAME: &'static str = "BindlessStorageWorkaroundVec4f32";
}

impl<T> ShaderNodeSingleType for BindlessStorageWorkaround<T>
where
  T: ShaderSizedValueNodeType + BindlessStorageWorkaroundNameHack,
{
  const SINGLE_TYPE: ShaderValueSingleType = ShaderValueSingleType::Unsized(
    ShaderUnSizedValueType::UnsizedStruct(&ShaderUnSizedStructMetaInfo {
      name: T::NAME,
      sized_fields: &[],
      last_dynamic_array_field: ("array", &T::MEMBER_TYPE),
    }),
  );
}

// we should impl this but for simplicity we skipped
// impl<T: ShaderSizedValueNodeType> ShaderUnsizedStructuralNodeType for
// BindlessStorageWorkaround<T> {
//   type Instance = ();
//   fn meta_info() -> &'static ShaderUnSizedStructMetaInfo {
//     &ShaderUnSizedStructMetaInfo {
//       name: "BindlessStorageWorkaround",
//       sized_fields: &[],
//       last_dynamic_array_field: (&"array", &T::MEMBER_TYPE),
//     }
//   }
// }

impl<T: ShaderSizedValueNodeType> ShaderMaybeUnsizedValueNodeType for BindlessStorageWorkaround<T> {
  const MAYBE_UNSIZED_TYPE: MaybeUnsizedValueType = todo!();
}

unsafe impl<T: Std430> Std430MaybeUnsized for BindlessStorageWorkaround<T> {
  fn bytes(&self) -> &[u8] {
    self.array.bytes()
  }

  fn from_bytes_into_boxed(bytes: &[u8]) -> Box<Self> {
    let new = Vec::from_iter(bytes.iter().copied()).into_boxed_slice();
    unsafe { std::mem::transmute(new) }
  }
}
