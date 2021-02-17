use crate::shading::copy::CopyParam;
use crate::{shading::*, vox::block::BlockFace};
use image::*;
use render_target::{RenderTarget, RenderTargetAble};
use rendiation_ral::{BindGroupCreator, Drawcall, ResourceManager, TextureHandle, Viewport, RAL};
use rendiation_renderable_mesh::{geometry::IndexedGeometry, tessellation::*};
use rendiation_webgpu::*;
use std::{collections::HashMap, sync::Arc};

pub struct BlockMetaInfo {
  name: String,
  id: usize,
  top_texture: Arc<BlockFaceTextureInfo>,
  bottom_texture: Arc<BlockFaceTextureInfo>,
  x_max_texture: Arc<BlockFaceTextureInfo>,
  x_min_texture: Arc<BlockFaceTextureInfo>,
  z_max_texture: Arc<BlockFaceTextureInfo>,
  z_min_texture: Arc<BlockFaceTextureInfo>,
}

impl BlockMetaInfo {
  pub fn get_uv_info(&self, face: BlockFace) -> [[f32; 2]; 4] {
    match face {
      BlockFace::XYMax => self.z_max_texture.uv,
      BlockFace::XYMin => self.z_min_texture.uv,
      BlockFace::YZMax => self.x_max_texture.uv,
      BlockFace::YZMin => self.x_min_texture.uv,
      BlockFace::XZMax => self.top_texture.uv,
      BlockFace::XZMin => self.bottom_texture.uv,
    }
  }
}

// struct TextureAtlas {
//   width_all: usize,
//   height_all: usize,
// }

struct NormalizedTexturePackInfo {
  x: f32,
  y: f32,
  w: f32,
  h: f32,
}

struct BlockFaceTextureInfo {
  pub img: DynamicImage,
  pub pack_info: NormalizedTexturePackInfo,
  pub uv: [[f32; 2]; 4],
}

impl BlockFaceTextureInfo {
  pub fn new(path: &str, uv: (f32, f32, f32, f32)) -> Self {
    let img = image::open(path).unwrap();
    let pack_info = NormalizedTexturePackInfo {
      x: uv.0,
      y: uv.1,
      w: uv.2,
      h: uv.3,
    };
    let uv = [
      [pack_info.x, pack_info.y],
      [pack_info.x + pack_info.w, pack_info.y],
      [pack_info.x, pack_info.y + pack_info.h],
      [pack_info.x + pack_info.w, pack_info.y + pack_info.h],
    ];
    BlockFaceTextureInfo { img, pack_info, uv }
  }
}

pub struct BlockRegistry {
  data: HashMap<String, Arc<BlockMetaInfo>>,
  pub lut: Vec<Arc<BlockMetaInfo>>,
}

impl BlockRegistry {
  pub fn new() -> Self {
    let data = HashMap::new();
    let lut = Vec::new();
    BlockRegistry { data, lut }
  }

  pub fn new_default() -> Self {
    let mut re = BlockRegistry::new();

    fn load_img(p: &str, uv: (f32, f32, f32, f32)) -> Arc<BlockFaceTextureInfo> {
      Arc::new(BlockFaceTextureInfo::new(p, uv))
    }
    let img = load_img("voxland/src/vox/assets/stone.png", (0.0, 0.0, 0.5, 0.5));

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

    let dirt = load_img("voxland/src/vox/assets/dirt.png", (0.5, 0.0, 0.5, 0.5));
    let dirt_block = BlockMetaInfo {
      name: String::from("stone"),
      id: 0,
      top_texture: dirt.clone(),
      bottom_texture: dirt.clone(),
      x_max_texture: dirt.clone(),
      x_min_texture: dirt.clone(),
      z_max_texture: dirt.clone(),
      z_min_texture: dirt.clone(),
    };
    re.register_block(dirt_block);

    let grass_top = load_img("voxland/src/vox/assets/grass_top.png", (0.0, 0.5, 0.5, 0.5));
    let grass_side = load_img(
      "voxland/src/vox/assets/grass_side.png",
      (0.5, 0.5, 0.5, 0.5),
    );

    let dirt_block = BlockMetaInfo {
      name: String::from("stone"),
      id: 0,
      top_texture: grass_top.clone(),
      bottom_texture: dirt.clone(),
      x_max_texture: grass_side.clone(),
      x_min_texture: grass_side.clone(),
      z_max_texture: grass_side.clone(),
      z_min_texture: grass_side.clone(),
    };
    re.register_block(dirt_block);

    re
  }

  pub fn register_block(&mut self, mut block: BlockMetaInfo) -> &mut Self {
    block.id = self.lut.len();
    let b = Arc::new(block);
    self.lut.push(b.clone());
    self.data.insert(b.name.clone(), b);
    self
  }

  pub fn create_atlas(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WebGPU>,
  ) -> WGPUTexture {
    // todo!();
    // todo filter same face
    let mut face_list: Vec<Arc<BlockFaceTextureInfo>> = Vec::new();
    face_list.push(self.lut[0].top_texture.clone());
    face_list.push(self.lut[1].top_texture.clone());
    face_list.push(self.lut[2].top_texture.clone());
    face_list.push(self.lut[2].x_max_texture.clone());

    pub fn tex(
      img_d: &DynamicImage,
      renderer: &mut WGPURenderer,
      resource: &mut ResourceManager<WebGPU>,
    ) -> TextureHandle<WebGPU> {
      let img = img_d.as_rgba8().unwrap().clone();
      let size = (img.width(), img.height(), 1);
      let data = img.into_raw();
      let texture = WGPUTexture::new_from_image_data(renderer, &data, size);
      resource.bindable.textures.insert(texture)
    }

    use rendiation_ral::GeometryResourceInstanceCreator;
    use rendiation_renderable_mesh::geometry::TriangleList;
    let quad = Quad.tessellate().geometry;
    let quad = IndexedGeometry::<_, _, TriangleList>::from(quad);
    let quad = quad.create_resource_instance_handle(renderer, resource);
    let sampler = WGPUSampler::default(renderer);
    let sampler = resource.bindable.samplers.insert(sampler);
    let target_texture = WGPUTexture::new_as_target_default(&renderer, (64, 64));
    let target = RenderTarget::from_one_texture(target_texture);

    let mut textures = Vec::new();
    let mut bindgroups = Vec::new();
    let mut shadings = Vec::new();

    {
      let dest_size_width = 64.;

      let gpu: Vec<_> = face_list
        .iter()
        .map(|face| {
          let src_tex = tex(&face.img, renderer, resource);
          textures.push(src_tex);
          let params = CopyParam::create_resource_instance(src_tex, sampler);
          let params = resource.add_bindgroup(params);
          bindgroups.push(params);
          let copy_shading = CopyShader::create_resource_instance(params);
          let copy_shading = resource
            .shadings
            .add_shading::<CopyShader>(copy_shading, renderer);
          shadings.push(copy_shading);

          let mut viewport = Viewport::new((32, 32));
          viewport.x = face.pack_info.x * dest_size_width;
          viewport.y = face.pack_info.y * dest_size_width;
          viewport.w = face.pack_info.w * dest_size_width;
          viewport.h = face.pack_info.h * dest_size_width;
          (copy_shading, viewport)
        })
        .collect();

      resource.maintain_gpu(renderer);

      let mut pass = target
        .create_render_pass_builder()
        .first_color(|c| c.load_with_clear((0., 0., 0.).into(), 1.0).ok())
        .create(renderer);

      for (shading, viewport) in &gpu {
        pass.use_viewport(&viewport);
        WebGPU::render_drawcall(
          &Drawcall::new(quad, *shading),
          unsafe { std::mem::transmute(&mut pass) },
          resource,
        );
      }
    }

    resource.bindable.samplers.remove(sampler);
    resource.delete_geometry_with_buffers(quad);
    textures.drain(..).for_each(|t| {
      resource.bindable.textures.remove(t);
    });
    bindgroups.drain(..).for_each(|t| {
      resource.delete_bindgroup(t);
    });
    shadings.drain(..).for_each(|t| {
      resource.shadings.delete_shading(t);
    });

    let (mut t, _) = target.dissemble();
    t.remove(0)
  }
}
