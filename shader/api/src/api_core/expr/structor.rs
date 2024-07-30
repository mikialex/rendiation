use crate::*;

impl<T> Node<T>
where
  T: ShaderStructuralNodeType,
{
  pub fn expand(self) -> T::Instance {
    T::expand(self)
  }
}

/// # Safety
///
/// the field index should be bounded and with correct type
///
/// .
pub unsafe fn expand_single<T>(struct_node: ShaderNodeRawHandle, field_index: usize) -> Node<T>
where
  T: ShaderNodeType,
{
  ShaderNodeExpr::FieldGet {
    field_index,
    struct_node,
  }
  .insert_api()
}

/// use for compile time ubo field reflection by procedure macro;
#[derive(Debug)]
pub struct ShaderStructMetaInfo {
  pub name: &'static str,
  pub fields: &'static [ShaderStructFieldMetaInfo],
}

impl ShaderStructMetaInfo {
  pub fn to_owned(&self) -> ShaderStructMetaInfoOwned {
    ShaderStructMetaInfoOwned {
      name: self.name.to_owned(),
      fields: self.fields.iter().map(|f| f.to_owned()).collect(),
    }
  }
}

impl PartialEq for ShaderStructMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}
impl Eq for ShaderStructMetaInfo {}
impl Hash for ShaderStructMetaInfo {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderInterpolation {
  Perspective,
  Linear,
  Flat,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderFieldDecorator {
  BuiltIn(ShaderBuiltInDecorator),
  Location(usize, Option<ShaderInterpolation>),
}

/// This trait is to mapping the real struct ty into the rendiation_shader_api node ty.
/// These types may be different because the std140 type substitution
pub trait ShaderFieldTypeMapper {
  type ShaderType: ShaderSizedValueNodeType;
}

// Impl notes:
//
// impl<T: ShaderSizedValueNodeType> ShaderFieldTypeMapper for T {
//   type ShaderType = T;
// }
//
// The reason we can not use this(above) with default ShaderType specialization is
//  the compiler can't infer this type equality:
// `let v: <rendiation_algebra::Vec4<f32> as ShaderFieldTypeMapper>::ShaderType = Vec4::default();`
//
//  So we have to impl for all the types we know

macro_rules! shader_field_ty_mapper {
  ($src:ty, $dst:ty) => {
    impl ShaderFieldTypeMapper for $src {
      type ShaderType = $dst;
    }
  };
}

// standard
shader_field_ty_mapper!(f32, Self);
shader_field_ty_mapper!(u32, Self);
shader_field_ty_mapper!(i32, Self);
shader_field_ty_mapper!(Vec2<f32>, Self);
shader_field_ty_mapper!(Vec3<f32>, Self);
shader_field_ty_mapper!(Vec4<f32>, Self);
shader_field_ty_mapper!(Vec2<u32>, Self);
shader_field_ty_mapper!(Vec3<u32>, Self);
shader_field_ty_mapper!(Vec4<u32>, Self);
shader_field_ty_mapper!(Mat2<f32>, Self);
shader_field_ty_mapper!(Mat3<f32>, Self);
shader_field_ty_mapper!(Mat4<f32>, Self);

// std140
shader_field_ty_mapper!(Shader16PaddedMat2, Mat2<f32>);
shader_field_ty_mapper!(Shader16PaddedMat3, Mat3<f32>);
shader_field_ty_mapper!(Bool, bool);

impl<T: ShaderSizedValueNodeType, const U: usize> ShaderFieldTypeMapper for Shader140Array<T, U> {
  type ShaderType = [T; U];
}

#[derive(Debug)]
pub struct ShaderStructFieldMetaInfo {
  pub name: &'static str,
  pub ty: ShaderSizedValueType,
  pub ty_deco: Option<ShaderFieldDecorator>,
}

impl ShaderStructFieldMetaInfo {
  pub fn to_owned(&self) -> ShaderStructFieldMetaInfoOwned {
    ShaderStructFieldMetaInfoOwned {
      name: self.name.to_owned(),
      ty: self.ty.clone(),
      ty_deco: self.ty_deco,
    }
  }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct ShaderStructFieldMetaInfoOwned {
  pub name: String,
  pub ty: ShaderSizedValueType,
  pub ty_deco: Option<ShaderFieldDecorator>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct ShaderStructMetaInfoOwned {
  pub name: String,
  pub fields: Vec<ShaderStructFieldMetaInfoOwned>,
}

impl ShaderStructMetaInfoOwned {
  pub fn new(name: &str) -> Self {
    Self {
      name: name.to_owned(),
      fields: Default::default(),
    }
  }

  pub fn push_field_dyn(&mut self, name: &str, ty: ShaderSizedValueType) {
    self.fields.push(ShaderStructFieldMetaInfoOwned {
      name: name.to_owned(),
      ty,
      ty_deco: None,
    });
  }

  #[must_use]
  pub fn add_field<T: ShaderSizedValueNodeType>(mut self, name: &str) -> Self {
    self.push_field_dyn(name, T::sized_ty());
    self
  }
}

#[derive(Debug)]
pub struct ShaderUnSizedStructMetaInfo {
  pub name: &'static str,
  pub sized_fields: &'static [ShaderStructFieldMetaInfo],
  /// according to spec, only unsized array is supported, unsized struct is not
  ///
  /// https://www.w3.org/TR/WGSL/#struct-types
  pub last_dynamic_array_field: (&'static str, &'static ShaderSizedValueType),
}

impl PartialEq for ShaderUnSizedStructMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}
impl Eq for ShaderUnSizedStructMetaInfo {}
impl Hash for ShaderUnSizedStructMetaInfo {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}
