use image::DynamicImage;
use image::ImageResult;
use image::{ImageBuffer, Rgba};
use rendiation::*;
use std::collections::HashMap;
use std::rc::Rc;
use crate::shading::*;

pub struct BlockMetaInfo {
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
  width_all: usize,
  height_all: usize,
}

struct NormalizedTexturePackInfo {
  x: f32,
  y: f32,
  w: f32,
  h: f32,
}

struct BlockFaceTextureInfo {
  pub img: DynamicImage,
  pub pack_info: NormalizedTexturePackInfo,
}

impl BlockFaceTextureInfo {
  pub fn new(path: &str) -> Self {
    let img = image::open(path).unwrap();
    let pack_info = NormalizedTexturePackInfo {
      x: 0.0,
      y: 0.0,
      w: 0.5,
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

    fn load_img(p: &str) -> Rc<BlockFaceTextureInfo> {
      Rc::new(BlockFaceTextureInfo::new(p))
    }
    let img = load_img("rinecraft/src/vox/assets/stone.png");

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

    // let dirt = load_img("rinecraft/src/vox/assets/dirt.png");
    // let dirt_block = BlockMetaInfo {
    //   name: String::from("stone"),
    //   id: 0,
    //   top_texture: dirt.clone(),
    //   bottom_texture: dirt.clone(),
    //   x_max_texture: dirt.clone(),
    //   x_min_texture: dirt.clone(),
    //   z_max_texture: dirt.clone(),
    //   z_min_texture: dirt.clone(),
    // };
    // re.register_block(dirt_block);

    re
  }

  pub fn register_block(&mut self, mut block: BlockMetaInfo) -> &mut Self {
    block.id = self.lut.len();
    let b = Rc::new(block);
    self.lut.push(b.clone());
    self.data.insert(b.name.clone(), b);
    self
  }

  pub fn create_atlas(&self, renderer: &mut WGPURenderer) -> WGPUTexture {
    // let imgd = image::open("rinecraft/src/vox/assets/stone.png").unwrap();
    // let img = imgd.as_rgba8().unwrap().clone();
    // let size = (img.width(),  img.height(), 1);
    // let data = img.into_raw();
    // WGPUTexture::new_from_image_data(&renderer.device, &mut renderer.encoder, &data,size)


    pub fn tex(path: &str, renderer: &mut WGPURenderer) -> WGPUTexture {
      let imgd = image::open(path).unwrap();
      let img = imgd.as_rgba8().unwrap().clone();
      let size = (img.width(),  img.height(), 1);
      let data = img.into_raw();
      WGPUTexture::new_from_image_data(&renderer.device, &mut renderer.encoder, &data,size)
    }

    let target_texture = WGPUTexture::new_as_target(&renderer.device, (64, 32, 1));
    {
      let quad = StandardGeometry::new_pair(quad_maker(), renderer);
      let sampler = WGPUSampler::new(&renderer.device);
      let copy_shading = CopierShading::new(renderer, &target_texture);
      let src_tex = tex("rinecraft/src/vox/assets/stone.png", renderer);
      let params = CopyShadingParamGroup::new(renderer, &copy_shading, src_tex.view(), &sampler);
  
      let mut pass = WGPURenderPass::build()
        .output_with_clear(target_texture.view(), (0., 0., 0., 1.0))
        .create(&mut renderer.encoder);
  
      // pass.use_viewport(&state.viewport);
      copy_shading.provide_pipeline(&mut pass, &params);
      quad.render(&mut pass);
    }

    target_texture
  }
}
