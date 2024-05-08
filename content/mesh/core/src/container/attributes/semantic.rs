use crate::*;

/// Vertex attribute semantic name.
///
/// https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Default)]
pub enum AttributeSemantic {
  /// XYZ vertex positions.
  #[default]
  Positions,

  /// XYZ vertex normals.
  Normals,

  /// XYZW vertex tangents where the `w` component is a sign value indicating the
  /// handedness of the tangent basis.
  Tangents,

  /// RGB or RGBA vertex color.
  Colors(u32),

  /// UV texture co-ordinates.
  TexCoords(u32),

  /// Joint indices.
  Joints(u32),

  /// Joint weights.
  Weights(u32),

  Foreign(ForeignAttributeKey),
}

pub trait AttributeReadSchema {
  fn item_byte_size(&self) -> usize;
}
define_dyn_trait_downcaster_static!(AttributeReadSchema);

impl AttributeReadSchema for AttributeSemantic {
  fn item_byte_size(&self) -> usize {
    match self {
      AttributeSemantic::Positions => 3 * 4,
      AttributeSemantic::Normals => 3 * 4,
      AttributeSemantic::Tangents => 4 * 4,
      AttributeSemantic::Colors(_) => 4 * 4,
      AttributeSemantic::TexCoords(_) => 2 * 4,
      AttributeSemantic::Joints(_) => 4 * 2,
      AttributeSemantic::Weights(_) => 4 * 4,
      AttributeSemantic::Foreign(key) => get_dyn_trait_downcaster_static!(AttributeReadSchema)
        .downcast_ref(key.implementation.as_ref())
        .unwrap() // this is safe to unwrap, because it's bounded in ForeignAttributeKey new method
        .item_byte_size(),
    }
  }
}

#[derive(Clone)]
pub struct ForeignAttributeKey {
  id: TypeId,
  pub implementation: Arc<dyn Any + Send + Sync>,
}

impl ForeignAttributeKey {
  pub fn new<T>(implementation: T) -> Self
  where
    T: std::any::Any
      + Clone
      + Send
      + Sync
      + AsRef<dyn AttributeReadSchema>
      + AsMut<dyn AttributeReadSchema>,
  {
    get_dyn_trait_downcaster_static!(AttributeReadSchema).register::<T>();
    Self {
      id: implementation.type_id(),
      implementation: Arc::new(implementation),
    }
  }
}

impl std::fmt::Debug for ForeignAttributeKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ForeignAttributeKey")
      .field("id", &self.id)
      .finish()
  }
}

impl Eq for ForeignAttributeKey {}
impl PartialEq for ForeignAttributeKey {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Ord for ForeignAttributeKey {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.id.cmp(&other.id)
  }
}
impl PartialOrd for ForeignAttributeKey {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl std::hash::Hash for ForeignAttributeKey {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}
