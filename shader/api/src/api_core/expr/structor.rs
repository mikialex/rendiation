use crate::*;

impl<T> Node<T>
where
  T: ShaderStructuralNodeType,
{
  pub fn expand(self) -> T::Instance {
    T::expand(self)
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
  fn into_shader_ty(self) -> Self::ShaderType;
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
  ($src:ty) => {
    impl ShaderFieldTypeMapper for $src {
      type ShaderType = Self;
      fn into_shader_ty(self) -> Self::ShaderType {
        self
      }
    }
  };
}

// standard
shader_field_ty_mapper!(f32);
shader_field_ty_mapper!(u32);
shader_field_ty_mapper!(i32);
shader_field_ty_mapper!(Vec2<f32>);
shader_field_ty_mapper!(Vec3<f32>);
shader_field_ty_mapper!(Vec4<f32>);
shader_field_ty_mapper!(Vec2<u32>);
shader_field_ty_mapper!(Vec3<u32>);
shader_field_ty_mapper!(Vec4<u32>);
shader_field_ty_mapper!(Mat2<f32>);
shader_field_ty_mapper!(Mat3<f32>);
shader_field_ty_mapper!(Mat4<f32>);

// std140
impl ShaderFieldTypeMapper for Shader16PaddedMat2 {
  type ShaderType = Mat2<f32>;
  fn into_shader_ty(self) -> Self::ShaderType {
    self.into()
  }
}

impl ShaderFieldTypeMapper for Shader16PaddedMat3 {
  type ShaderType = Mat3<f32>;
  fn into_shader_ty(self) -> Self::ShaderType {
    self.into()
  }
}

impl ShaderFieldTypeMapper for Bool {
  type ShaderType = bool;
  fn into_shader_ty(self) -> Self::ShaderType {
    self.0 != 0
  }
}

impl<T: ShaderSizedValueNodeType, const U: usize> ShaderFieldTypeMapper for Shader140Array<T, U> {
  type ShaderType = [T; U];
  fn into_shader_ty(self) -> Self::ShaderType {
    self.inner.map(|v| v.inner)
  }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct ShaderStructFieldMetaInfo {
  pub name: String,
  pub ty: ShaderSizedValueType,
  pub ty_deco: Option<ShaderFieldDecorator>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct ShaderStructMetaInfo {
  pub name: String,
  pub fields: Vec<ShaderStructFieldMetaInfo>,
}

impl ShaderStructMetaInfo {
  pub fn new(name: &str) -> Self {
    Self {
      name: name.to_owned(),
      fields: Default::default(),
    }
  }

  pub fn push_field_dyn(&mut self, name: &str, ty: ShaderSizedValueType) {
    self.fields.push(ShaderStructFieldMetaInfo {
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
  pub name: String,
  pub sized_fields: Vec<ShaderStructFieldMetaInfo>,
  /// according to spec, only unsized array is supported, unsized struct is not
  ///
  /// https://www.w3.org/TR/WGSL/#struct-types
  pub last_dynamic_array_field: (String, Box<ShaderSizedValueType>),
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
