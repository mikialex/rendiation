// https://users.rust-lang.org/t/proc-macro-referencing-self-module/49582
use crate as rendiation_shader_api;
use crate::*;

#[derive(Debug, Clone, Copy, ShaderStruct)]
pub struct HighPrecisionTranslation {
  pub f1: Vec3<f32>,
  pub f2: Vec3<f32>,
}

#[repr(C)]
#[std140_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, Default, PartialEq)]
pub struct HighPrecisionTranslationUniform {
  pub f1: Vec3<f32>,
  pub f2: Vec3<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, Default, PartialEq)]
pub struct HighPrecisionTranslationStorage {
  pub f1: Vec3<f32>,
  pub f2: Vec3<f32>,
}

pub fn into_mat_hpt_pair(mat: Mat4<f64>) -> (Mat4<f32>, HighPrecisionTranslation) {
  let position = mat.position();
  let f1 = position.into_f32();
  let f2 = (position - f1.into_f64()).into_f32();

  let hpt = HighPrecisionTranslation { f1, f2 };

  (mat.remove_position().into_f32(), hpt)
}

pub fn into_mat_hpt_uniform_pair(mat: Mat4<f64>) -> (Mat4<f32>, HighPrecisionTranslationUniform) {
  let (mat, hpt) = into_mat_hpt_pair(mat);

  let hpt = HighPrecisionTranslationUniform {
    f1: hpt.f1,
    f2: hpt.f1,
    ..Default::default()
  };

  (mat, hpt)
}

pub fn into_mat_hpt_storage_pair(mat: Mat4<f64>) -> (Mat4<f32>, HighPrecisionTranslationStorage) {
  let (mat, hpt) = into_mat_hpt_pair(mat);

  let hpt = HighPrecisionTranslationStorage {
    f1: hpt.f1,
    f2: hpt.f1,
    ..Default::default()
  };

  (mat, hpt)
}

pub fn hpt_uniform_to_hpt(
  hpt: Node<HighPrecisionTranslationUniform>,
) -> Node<HighPrecisionTranslation> {
  let hpt = hpt.expand();
  ENode::<HighPrecisionTranslation> {
    f1: hpt.f1,
    f2: hpt.f2,
  }
  .construct()
}

pub fn hpt_storage_to_hpt(
  hpt: Node<HighPrecisionTranslationStorage>,
) -> Node<HighPrecisionTranslation> {
  let hpt = hpt.expand();
  ENode::<HighPrecisionTranslation> {
    f1: hpt.f1,
    f2: hpt.f2,
  }
  .construct()
}

pub fn hpt_sub_hpt(
  hpt1: Node<HighPrecisionTranslation>,
  hpt2: Node<HighPrecisionTranslation>,
) -> Node<Vec3<f32>> {
  let hpt1 = hpt1.expand();
  let hpt2 = hpt2.expand();
  // todo, make sure shader compiler not optimize(reorder) this expression
  // https://github.com/gpuweb/gpuweb/issues/2076
  // currently we have issues on Metal
  (hpt1.f1 - hpt2.f1) + (hpt1.f2 - hpt2.f2)
}
