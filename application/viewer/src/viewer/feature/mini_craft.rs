//! a minecraft like demo to demonstrate some features

use egui_wgpu::wgpu::naga::FastHashMap;
use winit::keyboard::KeyCode;

use crate::*;

pub fn use_mini_craft_demo(cx: &mut ViewerCx) {
  let (cx, world) = cx.use_plain_state_init(|_| World {
    chunks: Default::default(),
    block_meta: vec![
      BlockTypeMeta {
        color: Vec3::splat(1.),
        void: true,
      },
      BlockTypeMeta {
        color: Vec3::splat(1.),
        void: false,
      },
    ],
  });

  match &mut cx.stage {
    ViewerCxStage::EventHandling { picker, input, .. } => {
      let ray = picker.current_mouse_ray_in_world();
      let max_distance = 20.;

      if input
        .window_state
        .pressed_keys
        .contains(&KeyCode::ControlLeft)
      {
        if input.window_state.is_left_mouse_pressed() {
          world.add_block_by_ray(1, ray, max_distance);
        } else if input.window_state.is_right_mouse_pressed() {
          world.delete_block_by_ray(ray, max_distance);
        }
      }
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      world.update_scene(writer);
    }
    _ => {}
  }
}

struct World {
  chunks: FastHashMap<(i32, i32, i32), WorldChunk>,
  block_meta: Vec<BlockTypeMeta>,
}

impl World {
  fn add_block_by_ray(&mut self, block_ty: u32, ray: Ray3, max_distance: f32) {
    if let Some((_, block_id)) = self.raycast(ray, max_distance) {
      //
    }
  }

  fn delete_block_by_ray(&mut self, ray: Ray3, max_distance: f32) {
    if let Some((_, block_id)) = self.raycast(ray, max_distance) {
      //
    }
  }

  fn raycast(&self, ray: Ray3, max_distance: f32) -> Option<(BlockFace, (i64, i64, i64))> {
    todo!()
  }

  fn update_scene(&mut self, writer: &mut SceneWriter) {
    todo!()
  }

  fn check_block_face_visibility(&self, face: BlockFace, block_id: (i64, i64, i64)) -> bool {
    let (x, y, z) = block_id;
    let side_block = match face {
      BlockFace::XZMin => (x, y - 1, z),
      BlockFace::XZMax => (x, y + 1, z),
      BlockFace::XYMin => (x, y, z - 1),
      BlockFace::XYMax => (x, y, z + 1),
      BlockFace::YZMin => (x - 1, y, z),
      BlockFace::YZMax => (x + 1, y, z),
    };
    let self_ty = self.get_block_meta(block_id);
    let side_ty = self.get_block_meta(side_block);

    let self_is_solid = self_ty != 0;
    let side_is_solid = side_ty != 0;

    self_is_solid != side_is_solid
  }

  fn get_block_meta(&self, block_id: (i64, i64, i64)) -> u32 {
    let chunk_id = (
      (block_id.0 / CHUNK_WIDTH as i64) as i32,
      (block_id.1 / CHUNK_WIDTH as i64) as i32,
      (block_id.2 / CHUNK_WIDTH as i64) as i32,
    );

    let block_id_in_chunk = (
      block_id.0 % CHUNK_WIDTH as i64,
      block_id.1 % CHUNK_WIDTH as i64,
      block_id.2 % CHUNK_WIDTH as i64,
    );

    if let Some(chunk) = self.chunks.get(&chunk_id) {
      chunk.get_block(block_id_in_chunk)
    } else {
      0
    }
  }

  fn set_block_meta(&mut self, block_id: (i64, i64, i64), ty: u32) {
    let chunk_id = (
      (block_id.0 / CHUNK_WIDTH as i64) as i32,
      (block_id.1 / CHUNK_WIDTH as i64) as i32,
      (block_id.2 / CHUNK_WIDTH as i64) as i32,
    );

    let block_id_in_chunk = (
      block_id.0 % CHUNK_WIDTH as i64,
      block_id.1 % CHUNK_WIDTH as i64,
      block_id.2 % CHUNK_WIDTH as i64,
    );

    self
      .chunks
      .entry(chunk_id)
      .or_default()
      .set_block(block_id_in_chunk, ty);
  }
}

struct WorldChunk {
  data: Vec<u32>,
  model: Option<EntityHandle<SceneModelEntity>>,
}

impl WorldChunk {
  pub fn get_block(&self, block_id_in_chunk: (i64, i64, i64)) -> u32 {
    self.data[block_id_in_chunk.2 as usize * CHUNK_WIDTH * CHUNK_WIDTH
      + block_id_in_chunk.1 as usize * CHUNK_WIDTH
      + block_id_in_chunk.0 as usize]
  }
  pub fn set_block(&mut self, block_id_in_chunk: (i64, i64, i64), ty: u32) {
    self.data[block_id_in_chunk.2 as usize * CHUNK_WIDTH * CHUNK_WIDTH
      + block_id_in_chunk.1 as usize * CHUNK_WIDTH
      + block_id_in_chunk.0 as usize] = ty
  }
}

impl Default for WorldChunk {
  fn default() -> Self {
    Self {
      data: vec![0; CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH],
      model: Default::default(),
    }
  }
}

#[derive(Copy, Clone, PartialEq)]
pub enum BlockFace {
  XYMin,
  XYMax,
  XZMin,
  XZMax,
  YZMin,
  YZMax,
}

pub const BLOCK_WORLD_SIZE: f32 = 1.0;
pub const CHUNK_WIDTH: usize = 16;

struct BlockTypeMeta {
  color: Vec3<f32>,
  void: bool,
}
