use fast_hash_collection::FastHashMap;
use rendiation_texture_gpu_system::*;

use crate::*;

pub struct VirtualTextureSystem {
  config: VirtualTexturePageMetadata,
  physical_texture: GPU2DArrayTextureView,
  /// should use bindless on advance platform, in other case using pool
  /// of cause another virtual system can be used here if you are crazy
  page_mapping_system: Box<dyn DynAbstractGPUTextureSystem>,
  page_mapping_host: Vec<u32>,
  empty_pages: Vec<u32>,
}

impl VirtualTextureSystem {
  pub fn new(config: VirtualTexturePageMetadata, gpu: &GPU) -> Self {
    let virtual_page_count = config.virtual_page_count();
    assert!(virtual_page_count < u32::MAX);

    let physical_texture = GPUTexture::create(
      TextureDescriptor {
        label: "texture-pool".into(),
        size: config.gpu_extend(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: config.format,
        view_formats: &[],
        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
      },
      &gpu.device,
    )
    .create_view(TextureViewDescriptor {
      label: "texture pool view".into(),
      dimension: TextureViewDimension::D2Array.into(),
      ..Default::default()
    })
    .try_into()
    .unwrap();

    let page_mapping_host = vec![u32::MAX; virtual_page_count as usize];
    let empty_pages: Vec<u32> = (0..virtual_page_count).collect();

    Self {
      config,
      physical_texture,
      page_mapping_system: todo!(),
      page_mapping_host,
      empty_pages,
    }
  }
}

pub struct VirtualTexturePageMetadata {
  pub page_width: u32,
  pub physical_page_count_one_side: u32,
  pub physical_layer_count: u32,
  pub virtual_page_count_one_side: u32,
  pub virtual_layer_count: u32,
  pub format: TextureFormat,
}

impl VirtualTexturePageMetadata {
  pub fn physical_page_count(&self) -> u32 {
    self.physical_page_count_one_side
      * self.physical_page_count_one_side
      * self.physical_layer_count
  }
  pub fn physical_texel_count(&self) -> u32 {
    self.physical_page_count() * self.page_width * self.page_width
  }

  pub fn virtual_page_count(&self) -> u32 {
    self.virtual_page_count_one_side * self.virtual_page_count_one_side * self.virtual_layer_count
  }

  pub fn virtual_texel_count(&self) -> u32 {
    self.virtual_page_count() * self.page_width * self.page_width
  }

  pub fn gpu_extend(&self) -> Extent3d {
    let physical_texel_count_side = self.physical_page_count_one_side * self.page_width;

    Extent3d {
      width: physical_texel_count_side,
      height: physical_texel_count_side,
      depth_or_array_layers: self.physical_layer_count,
    }
  }
}

// pub struct IndirectTextureUsageCountedSystem<T> {
//   internal: T,
//   collector: DeviceUsageCounter,
// }

// impl<T: AbstractIndirectGPUTextureSystem> AbstractIndirectGPUTextureSystem
//   for IndirectTextureUsageCountedSystem<T>
// {
//   fn bind_system_self(&self, collector: &mut BindingBuilder) {
//     self.internal.bind_system_self(collector);
//     self.collector.bind(collector);
//   }

//   fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
//     self.internal.register_system_self(builder);
//     todo!();
//     // self.collector.build(builder);
//   }

//   fn register_system_self_for_compute(
//     &self,
//     builder: &mut ShaderBindGroupBuilder,
//     reg: &mut SemanticRegistry,
//   ) {
//     self.internal.register_system_self_for_compute(builder, reg);
//     todo!();
//     // self.collector.build(builder);
//   }

//   fn sample_texture2d_indirect(
//     &self,
//     reg: &SemanticRegistry,
//     shader_texture_handle: Node<Texture2DHandle>,
//     shader_sampler_handle: Node<SamplerHandle>,
//     uv: Node<Vec2<f32>>,
//   ) -> Node<Vec4<f32>> {
//     let collector: &DeviceUsageCounterInvocation = todo!();

//     // todo, skip 0 handle
//     // consider reduce the cost in trade of the correctness, for example randomly give up record.
//     // the worst case here is one texture quad draw in full frame cause great atomic contention.
//     collector.record_usage(shader_texture_handle);

//     self
//       .internal
//       .sample_texture2d_indirect(reg, shader_texture_handle, shader_sampler_handle, uv)
//   }
// }
