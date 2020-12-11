use crate::WebGLTexture;
use web_sys::*;

pub struct TextureSlotStates {
  slot: u32,
  slots: Vec<Option<TextureSlotBindInfo>>,
  active_slot: Option<u32>,
  max_support_slot: u32,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebGLTextureBindType {
  Texture2D = WebGl2RenderingContext::TEXTURE_BINDING_2D,
  TextureCubeMap = WebGl2RenderingContext::TEXTURE_BINDING_CUBE_MAP,
}

impl TextureSlotStates {
  pub fn new(max_combined_texture_image_units: u32) -> Self {
    Self {
      slot: 0,
      slots: vec![None; max_combined_texture_image_units as usize],
      active_slot: None,
      max_support_slot: max_combined_texture_image_units,
    }
  }

  pub fn active_texture(&mut self, slot: u32, gl: &WebGl2RenderingContext) {
    if let Some(active_slot) = self.active_slot {
      if active_slot == slot {
        return;
      }
    }
    gl.active_texture(slot);
    self.active_slot = Some(slot);
  }

  pub fn bind_texture(&mut self, texture: &WebGLTexture, gl: &WebGl2RenderingContext) {
    let bind_type = texture.ty;
    let texture_id = texture.id;
    let texture = &texture.texture;

    let active_slot = self.active_slot.unwrap_or_else(|| {
      let slot = WebGl2RenderingContext::TEXTURE0 + self.max_support_slot - 1;
      self.active_texture(slot, gl);
      slot
    });
    let slot_bound = &mut self.slots[active_slot as usize];
    if let Some(v) = slot_bound {
      if v.bind_type != bind_type || v.texture_id != texture_id {
        gl.bind_texture(bind_type as u32, Some(texture));
        *v = TextureSlotBindInfo {
          bind_type,
          texture_id,
        }
      }
    };

    slot_bound.get_or_insert_with(|| {
      gl.bind_texture(bind_type as u32, Some(texture));
      TextureSlotBindInfo {
        bind_type,
        texture_id,
      }
    });
  }

  pub fn reset_slots(&mut self) {
    self.slot = 0;
  }

  pub fn get_free_slot(&mut self) -> Option<u32> {
    let re = self.slot;
    self.slot += 1;
    if re > self.max_support_slot {
      None
    } else {
      Some(re)
    }
  }

  pub fn bind_and_active_texture(
    &mut self,
    texture: &WebGLTexture,
    gl: &WebGl2RenderingContext,
  ) -> u32 {
    let slot = self.get_free_slot().unwrap();
    self.active_texture(slot, gl);
    self.bind_texture(texture, gl);
    slot
  }
}

#[derive(Debug, Clone, Copy)]
pub struct TextureSlotBindInfo {
  bind_type: WebGLTextureBindType,
  texture_id: usize,
}
