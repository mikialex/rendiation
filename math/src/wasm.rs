use wasm_bindgen::prelude::*;

use crate::{Mat4, Vec2, Vec3, Vec4};

pub trait WASMAbleType {
  type Type;
  fn to_origin(self) -> Self::Type;
  fn from_origin(origin: Self::Type) -> Self;
}

#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec2F32WASM {
  pub x: f32,
  pub y: f32,
}

impl WASMAbleType for Vec2F32WASM {
  type Type = Vec2<f32>;
  fn to_origin(self) -> Self::Type {
    bytemuck::cast(self)
  }
  fn from_origin(origin: Self::Type) -> Self {
    bytemuck::cast(origin)
  }
}

unsafe impl bytemuck::Zeroable for Vec2F32WASM {}
unsafe impl bytemuck::Pod for Vec2F32WASM {}

#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec3F32WASM {
  pub x: f32,
  pub y: f32,
  pub z: f32,
}

impl WASMAbleType for Vec3F32WASM {
  type Type = Vec3<f32>;
  fn to_origin(self) -> Self::Type {
    bytemuck::cast(self)
  }
  fn from_origin(origin: Self::Type) -> Self {
    bytemuck::cast(origin)
  }
}

unsafe impl bytemuck::Zeroable for Vec3F32WASM {}
unsafe impl bytemuck::Pod for Vec3F32WASM {}

#[wasm_bindgen]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vec4F32WASM {
  pub x: f32,
  pub y: f32,
  pub z: f32,
  pub w: f32,
}

impl WASMAbleType for Vec4F32WASM {
  type Type = Vec4<f32>;
  fn to_origin(self) -> Self::Type {
    bytemuck::cast(self)
  }
  fn from_origin(origin: Self::Type) -> Self {
    bytemuck::cast(origin)
  }
}

unsafe impl bytemuck::Zeroable for Vec4F32WASM {}
unsafe impl bytemuck::Pod for Vec4F32WASM {}

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

impl WASMAbleType for Mat4F32WASM {
  type Type = Mat4<f32>;
  fn to_origin(self) -> Self::Type {
    bytemuck::cast(self)
  }
  fn from_origin(origin: Self::Type) -> Self {
    bytemuck::cast(origin)
  }
}

unsafe impl bytemuck::Zeroable for Mat4F32WASM {}
unsafe impl bytemuck::Pod for Mat4F32WASM {}
