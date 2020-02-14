use image::DynamicImage;
use image::ImageResult;
use image::{ImageBuffer, Rgba};
use std::collections::HashMap;
use std::rc::Rc;

struct BlockMetaInfo {
  name: String,
  id: usize,
  top_texture: Rc<BlockFaceTextureInfo>,
  bottom_texture: Rc<BlockFaceTextureInfo>,
  x_max_texture: Rc<BlockFaceTextureInfo>,
  x_min_texture: Rc<BlockFaceTextureInfo>,
  z_max_texture: Rc<BlockFaceTextureInfo>,
  z_min_texture: Rc<BlockFaceTextureInfo>,
}

struct TextureAtlas {
  // pack_info:
}

struct NormalizedTexturePackInfo {
  x: f32,
  y: f32,
  w: f32,
  h: f32,
}

struct BlockFaceTextureInfo {
  img: DynamicImage,
  pack_info: NormalizedTexturePackInfo,
}

impl BlockFaceTextureInfo {
  pub fn new(path: &str) -> Self {
    let img = image::open(path).unwrap();
    let pack_info = NormalizedTexturePackInfo {
      x: 0.0,
      y: 0.0,
      w: 1.0,
      h: 1.0,
    };
    BlockFaceTextureInfo { img, pack_info }
  }
}

impl TextureAtlas {}

pub struct BlockRegistry {
  data: HashMap<String, Rc<BlockMetaInfo>>,
  lut: Vec<Rc<BlockMetaInfo>>,
}

impl BlockRegistry {
  pub fn new() -> Self {
    let data = HashMap::new();
    let lut = Vec::new();
    BlockRegistry { data, lut }
  }

  pub fn new_default() -> Self {
    let mut re = BlockRegistry::new();
    let path = std::env::current_dir().unwrap();
    println!("The current directory is {}", path.display());
    let img = Rc::new(BlockFaceTextureInfo::new(
      "rinecraft/src/vox/assets/stone.png",
    ));
    let stone = BlockMetaInfo {
      name: String::from("stone"),
      id: 0,
      top_texture: img.clone(),
      bottom_texture: img.clone(),
      x_max_texture: img.clone(),
      x_min_texture: img.clone(),
      z_max_texture: img.clone(),
      z_min_texture: img.clone(),
    };
    re.register_block(stone);
    re
  }

  pub fn register_block(&mut self, mut block: BlockMetaInfo) -> &mut Self {
    block.id = self.lut.len();
    let b = Rc::new(block);
    self.lut.push(b.clone());
    self.data.insert(b.name.clone(), b);
    self
  }
}
