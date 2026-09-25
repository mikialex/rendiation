//! Declare the bindings once, and use the same declaration for both shader side and pass side.
//!
//! Traditionally the `bind_by` call order in shader building must be exactly same as the `bind`
//! call order in pass setup, which is an implicit contract and easy to break. By implementing
//! [BindingDeclare], the binding order is guaranteed by construction because both side execute
//! the same declare function, and the shader side still gets the fully typed shader instances.
//!
//! The [BindKind] decides what each bind item produces, the [Binder] does the actual binding.
//! They are separated because the binder holds a mutable borrow of the builder, if the result type
//! depends on the binder, the result will keep borrowing the builder.

use crate::*;

pub trait BindItem: AbstractShaderBindingSource + AbstractBindingSource {}
impl<T: AbstractShaderBindingSource + AbstractBindingSource> BindItem for T {}

pub trait BindKind {
  type Out<T: BindItem>;
}

/// bind in current shader stage, produce the shader instance
pub struct ShaderKind;
impl BindKind for ShaderKind {
  type Out<T: BindItem> = T::ShaderBindResult;
}

/// bind in both vertex(or mesh) and fragment stage, produce the accessor of both stage instance
pub struct GraphicsPairKind;
impl BindKind for GraphicsPairKind {
  type Out<T: BindItem> = GraphicsPairInputNodeAccessor<T>;
}

/// bind in pass, produce nothing
pub struct PassKind;
impl BindKind for PassKind {
  type Out<T: BindItem> = ();
}

pub trait Binder {
  type Kind: BindKind;
  fn bind<T: BindItem>(&mut self, item: &T) -> <Self::Kind as BindKind>::Out<T>;
  /// bind the items in the scope to the given bindgroup index
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R;
}

pub trait BindingDeclare {
  /// the typed bindings, usually a struct that each field is `K::Out<ItemType>`
  type Bindings<K: BindKind>;
  fn declare<B: Binder>(&self, b: &mut B) -> Self::Bindings<B::Kind>;
}

/// bind in current shader stage, must be used inside a shader stage
pub struct ShaderBinder<'a>(pub &'a mut ShaderBindGroupBuilder);
impl Binder for ShaderBinder<'_> {
  type Kind = ShaderKind;
  fn bind<T: BindItem>(&mut self, item: &T) -> T::ShaderBindResult {
    self.0.bind_by(item)
  }
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R {
    let before = self.0.set_binding_slot(group);
    let r = f(self);
    self.0.set_binding_slot(before);
    r
  }
}

/// bind in both vertex(or mesh) and fragment stage, must be used outside any shader stage.
///
/// note, the writeable storage buffer is not allowed in vertex stage, so such item should not be
/// bound by this binder.
pub struct GraphicsPairBinder<'a>(pub &'a mut ShaderRenderPipelineBuilder);
impl Binder for GraphicsPairBinder<'_> {
  type Kind = GraphicsPairKind;
  fn bind<T: BindItem>(&mut self, item: &T) -> GraphicsPairInputNodeAccessor<T> {
    BindingPreparer::new(item).using_graphics_pair(self.0, |_, _| {})
  }
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R {
    let before = self.0.set_binding_slot(group);
    let r = f(self);
    self.0.set_binding_slot(before);
    r
  }
}

pub struct PassBinder<'a>(pub &'a mut BindingBuilder);
impl Binder for PassBinder<'_> {
  type Kind = PassKind;
  fn bind<T: BindItem>(&mut self, item: &T) {
    self.0.bind(item);
  }
  fn with_group<R>(&mut self, group: usize, f: impl FnOnce(&mut Self) -> R) -> R {
    let before = self.0.set_binding_slot(group);
    let r = f(self);
    self.0.set_binding_slot(before);
    r
  }
}

#[cfg(test)]
mod sample {
  use crate::*;

  /// a leaf component with optional texture, only used in fragment stage
  pub struct TexturedMaterial {
    pub base_color: UniformBufferDataView<Vec4<f32>>,
    pub base_color_tex: Option<GPU2DTextureView>,
    pub sampler: GPUSamplerView,
  }

  pub struct TexturedMaterialBindings<K: BindKind> {
    pub base_color: K::Out<UniformBufferDataView<Vec4<f32>>>,
    pub tex: Option<(K::Out<GPU2DTextureView>, K::Out<GPUSamplerView>)>,
  }

  impl BindingDeclare for TexturedMaterial {
    type Bindings<K: BindKind> = TexturedMaterialBindings<K>;
    fn declare<B: Binder>(&self, b: &mut B) -> TexturedMaterialBindings<B::Kind> {
      TexturedMaterialBindings {
        base_color: b.bind(&self.base_color),
        tex: self
          .base_color_tex
          .as_ref()
          .map(|t| (b.bind(t), b.bind(&self.sampler))),
      }
    }
  }

  /// a composite component, shows nesting, bindgroup assignment and cross stage binding
  pub struct QuadWithMaterial {
    pub view_projection: UniformBufferDataView<Mat4<f32>>,
    pub material: TexturedMaterial,
  }

  pub struct QuadWithMaterialBindings<K: BindKind> {
    pub view_projection: K::Out<UniformBufferDataView<Mat4<f32>>>,
    pub material: TexturedMaterialBindings<K>,
  }

  impl BindingDeclare for QuadWithMaterial {
    type Bindings<K: BindKind> = QuadWithMaterialBindings<K>;
    fn declare<B: Binder>(&self, b: &mut B) -> QuadWithMaterialBindings<B::Kind> {
      QuadWithMaterialBindings {
        view_projection: b.with_group(0, |b| b.bind(&self.view_projection)),
        material: b.with_group(1, |b| self.material.declare(b)),
      }
    }
  }

  impl ShaderHashProvider for QuadWithMaterial {
    fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
      hasher.hash(self.material.base_color_tex.is_some());
    }
    shader_hash_type_id! {}
  }

  impl GraphicsShaderProvider for QuadWithMaterial {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
      // outside any stage, every binding is accessible in both vertex and fragment stage
      let b = self.declare(&mut GraphicsPairBinder(builder));

      builder.vertex(|builder, _| {
        *builder.primitive_state() = PrimitiveState {
          topology: PrimitiveTopology::TriangleStrip,
          front_face: FrontFace::Cw,
          ..Default::default()
        };
        let quad = generate_quad(builder.query::<VertexIndex>(), 0.).expand();
        let view_projection = b.view_projection.get().load();
        builder.register::<ClipPosition>(view_projection * quad.position);
        builder.set_vertex_out::<FragmentUv>(quad.uv);
      });

      builder.fragment(|builder, _| {
        let m = &b.material;
        let mut color = m.base_color.get().load();
        if let Some((tex, sampler)) = &m.tex {
          let uv = builder.query::<FragmentUv>();
          color *= tex.get().sample(sampler.get(), uv);
        }
        builder.register::<DefaultDisplay>(color);
      });
    }
  }

  impl ShaderPassBuilder for QuadWithMaterial {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
      self.declare(&mut PassBinder(&mut ctx.binding));
    }
  }

  async fn render_first_pixel(gpu: &GPU, with_tex: bool) -> Vec<u8> {
    let size = Size::from_u32_pair_min_one((4, 4));
    let usage =
      TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC | TextureUsages::TEXTURE_BINDING;
    let target_view = create_empty_2d_texture_view(gpu, size, usage, TextureFormat::Rgba8Unorm);
    let target = RenderTargetView::from(target_view.clone());

    let tex_usage = TextureUsages::TEXTURE_BINDING;
    let quad = QuadWithMaterial {
      view_projection: create_uniform(Mat4::identity(), gpu, "view_projection"),
      material: TexturedMaterial {
        base_color: create_uniform(Vec4::new(1., 0.5, 0., 1.), gpu, "base_color"),
        base_color_tex: with_tex
          .then(|| create_empty_2d_texture_view(gpu, size, tex_usage, TextureFormat::Rgba8Unorm)),
        sampler: gpu
          .device
          .create_and_cache_sampler_view(SamplerDescriptor::default()),
      },
    };

    let mut encoder = gpu.create_encoder();
    {
      let mut active = pass("binding declare sample")
        .with_color(&target, clear_and_store(all_zero()))
        .render(&mut encoder, gpu, None, None);
      // mix with the component that not using the binding declare
      let base = default_dispatcher(&active.pass, false);
      let components: [&dyn RenderComponent; 2] = [&base, &quad];
      RenderArray(components).render(
        &mut active.pass.ctx,
        RenderMethod::TraditionalDraw(QUAD_DRAW_CMD),
      );
    }
    let range = ReadRange {
      size: Size::from_u32_pair_min_one((1, 1)),
      offset_x: 0,
      offset_y: 0,
    };
    let read = encoder.read_texture_2d(&gpu.device, &target_view, range);
    gpu.submit_encoder(encoder);
    read.await.unwrap().read_into_raw_unpadded_buffer()
  }

  #[pollster::test]
  async fn test_binding_declare_sample() {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    assert_eq!(
      render_first_pixel(&gpu, false).await,
      vec![255, 128, 0, 255]
    );
    // the texture is zero initialized
    assert_eq!(render_first_pixel(&gpu, true).await, vec![0, 0, 0, 0]);
  }
}
