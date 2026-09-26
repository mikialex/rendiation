use rendiation_shader_api::*;

use crate::harness::*;

/// float texture sampling and query
#[test]
fn float_texture() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let tex_2d: BindingNode<ShaderTexture2D> = fake_binding(0);
    let tex_2d_array: BindingNode<ShaderTexture2DArray> = fake_binding(1);
    let tex_1d: BindingNode<ShaderTexture1D> = fake_binding(2);
    let sampler: BindingNode<ShaderSampler> = fake_binding(3);
    let uv = f.splat::<Vec2<f32>>();

    keep(tex_2d.sample_zero_level(sampler, uv));
    keep(
      tex_2d
        .build_sample_call(sampler, uv)
        .with_offset(Vec2::new(-8, 7))
        .with_level(f)
        .sample(),
    );
    keep(
      tex_2d
        .build_sample_call(sampler, uv)
        .with_level_grad(uv, uv)
        .sample(),
    );
    keep(tex_2d.sample_base_clamp_to_edge(sampler, uv));
    keep(tex_2d.texture_dimension_2d(Some(u)));
    keep(
      tex_2d_array
        .build_sample_call(sampler, uv)
        .with_array_index(u)
        .with_zero_level()
        .sample(),
    );
    keep(tex_2d_array.texture_number_layers());
    keep(tex_2d_array.texture_number_levels());

    // 1d texture coordinates are scalar
    keep(tex_1d.sample_zero_level(sampler, f));
    keep(tex_1d.load_texel(u, val(0)));
  });
}

/// integer texture can be loaded and gathered
#[test]
fn integer_texture() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let tex: BindingNode<ShaderTexture2DUint> = fake_binding(0);
    let sampler: BindingNode<ShaderSampler> = fake_binding(1);

    keep(tex.load_texel(u.splat(), val(0)));
    keep(
      tex
        .build_sample_call(sampler, f.splat())
        .gather(GatherChannel::W),
    );
  });
}

/// depth texture sampling and comparison
#[test]
fn depth_texture() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let depth: BindingNode<ShaderDepthTexture2D> = fake_binding(0);
    let depth_array: BindingNode<ShaderDepthTexture2DArray> = fake_binding(1);
    let sampler: BindingNode<ShaderSampler> = fake_binding(2);
    let compare_sampler: BindingNode<ShaderCompareSampler> = fake_binding(3);
    let uv = f.splat::<Vec2<f32>>();

    // the explicit level of depth texture is u32
    keep(depth.build_sample_call(sampler, uv).with_level(u).sample());
    keep(depth.sample_zero_level(sampler, uv));
    keep(
      depth
        .build_sample_call(sampler, uv)
        .gather(GatherChannel::X),
    );
    keep(
      depth
        .build_compare_sample_call(compare_sampler, uv, f)
        .sample(),
    );
    keep(
      depth_array
        .build_compare_sample_call(compare_sampler, uv, f)
        .with_offset(Vec2::new(1, 1))
        .with_array_index(u)
        .gather(),
    );
  });
}

/// storage texture query
#[test]
fn storage_texture() {
  check_compute(|builder| {
    let RuntimeValues { u, f, .. } = runtime_values(builder);
    let storage: BindingNode<
      ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension2Array, f32>,
    > = fake_binding(0);

    keep(storage.texture_number_layers());
    keep(storage.texture_dimension_2d());
    keep(storage.load_texel_layer(u.splat(), u));
    storage.write_texel_index(u.splat(), u, f.splat());
    texture_barrier();
  });
}

/// the offset component must be in [-8, 7]
#[test]
#[should_panic(expected = "texture sample offset component must be in the range [-8, 7]")]
fn sample_offset_out_of_range() {
  build_compute(|builder| {
    let f = runtime_values(builder).f;
    let tex: BindingNode<ShaderTexture2D> = fake_binding(0);
    let sampler: BindingNode<ShaderSampler> = fake_binding(1);
    tex
      .build_sample_call(sampler, f.splat())
      .with_offset(Vec2::new(8, 0));
  });
}

/// depth texture gather has no component parameter, the channel must be X
#[test]
#[should_panic(expected = "depth texture gather channel must be X")]
fn depth_gather_channel() {
  build_compute(|builder| {
    let f = runtime_values(builder).f;
    let depth: BindingNode<ShaderDepthTexture2D> = fake_binding(0);
    let sampler: BindingNode<ShaderSampler> = fake_binding(1);
    keep(
      depth
        .build_sample_call(sampler, f.splat())
        .gather(GatherChannel::Y),
    );
  });
}
