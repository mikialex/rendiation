use rendiation_math::*;
use wasm_bindgen::prelude::*;

use crate::{Box3, Ray3, Sphere};

#[wasm_bindgen]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Box3WASM {
  pub min: Vec3F32WASM,
  pub max: Vec3F32WASM,
}

#[wasm_bindgen]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SphereWASM {
  pub center: Vec3F32WASM,
  pub radius: f32,
}

#[wasm_bindgen]
#[repr(C)]
pub struct Ray3WASM {
  pub origin: Vec3F32WASM,
  pub direction: Vec3F32WASM,
}

#[wasm_bindgen]
impl Ray3WASM {
  #[wasm_bindgen]
  pub fn new() -> Self {
    Ray3::new(Vec3::zero(), Vec3::new(1.0, 0.0, 0.0)).to_wasm()
  }
}

macro_rules! impl_convert_unsafe {
  ($Origin: ty, $WASM: ty) => {
    impl WASMAbleType for $Origin {
      type Type = $WASM;
      fn to_wasm(self) -> Self::Type {
        unsafe { std::mem::transmute(self) }
      }
      fn from_wasm(ty: Self::Type) -> Self {
        unsafe { std::mem::transmute(ty) }
      }
    }
  };
}
impl_convert_unsafe!(Sphere, SphereWASM);
impl_convert_unsafe!(Box3, Box3WASM);
impl_convert_unsafe!(Ray3, Ray3WASM);
