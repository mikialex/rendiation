use webgpu::*;

use crate as shadergraph;
use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(ShaderStruct, Clone, Copy, Default)]
pub struct TestUniform {
  pub data: f32,
  pub data2: Vec2<f32>,
  pub data3: Vec3<f32>,
}

impl ShaderBindingProvider for TestUniform {
  type Node = Self;
}

impl ShaderGraphProvider for TestUniform {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.uniform::<Self>(SB::Object);

    builder.vertex(|builder, binding| {
      let uniform_a = binding
        .uniform::<ResourceViewRc<UniformBuffer<Shader140Array<TestUniform, 4>>>>(SB::Object);

      let mut_n: Node<Vec3<f32>> = Vec3::<f32>::new(0.0, 0.0, 0.0).into();
      let mut_n = mut_n.mutable();

      for_by(uniform_a, |ctx, node| {
        let node_idx = uniform_a.index(node);
        let data3 = node_idx.expand().data3;
        mut_n.set(data3 + temp);
      });
      builder.register::<ClipPosition>((mut_n.get(), 1.0));
      Ok(())
    })
  }
}
