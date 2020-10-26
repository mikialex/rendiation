use wasm_bindgen::prelude::*;

use crate::{Mat4, Vec2, Vec3, Vec4};

pub trait WASMAbleType {
  type Type;
  fn from_origin(self) -> Self::Type;
  fn to_origin(ty: Self::Type) -> Self;
}

macro_rules! impl_convert_bytemuck {
  ($Origin: ty, $WASM: ty) => {
    unsafe impl bytemuck::Zeroable for $WASM {}
    unsafe impl bytemuck::Pod for $WASM {}
    impl WASMAbleType for $Origin {
      type Type = $WASM;
      fn from_origin(self) -> Self::Type {
        bytemuck::cast(self)
      }
      fn to_origin(ty: Self::Type) -> Self {
        bytemuck::cast(ty)
      }
    }
  };
}

#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec2F32WASM {
  pub x: f32,
  pub y: f32,
}

#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec3F32WASM {
  pub x: f32,
  pub y: f32,
  pub z: f32,
}

#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec4F32WASM {
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub w: f32,
}

#[rustfmt::skip]
#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Mat4F32WASM {
	pub a1:f32, pub a2:f32, pub a3:f32, pub a4:f32,
	pub b1:f32, pub b2:f32, pub b3:f32, pub b4:f32,
	pub c1:f32, pub c2:f32, pub c3:f32, pub c4:f32,
	pub d1:f32, pub d2:f32, pub d3:f32, pub d4:f32,
}

impl_convert_bytemuck!(Vec2<f32>, Vec2F32WASM);
impl_convert_bytemuck!(Vec3<f32>, Vec3F32WASM);
impl_convert_bytemuck!(Vec4<f32>, Vec4F32WASM);
impl_convert_bytemuck!(Mat4<f32>, Mat4F32WASM);
