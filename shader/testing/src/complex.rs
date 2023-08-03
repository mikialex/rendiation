use super::function::ACESFilmicToneMapping;
use crate::*;

struct Test;

impl GraphicsShaderProvider for Test {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      let a = consts(1.) + consts(2.);
      let a: Node<_> = (Vec3::zero(), a).into();
      let position = builder.query_or_insert_default::<ClipPosition>().mutable();
      position.set(a);

      position.set(Vec4::zero());

      let a = consts(1.).mutable();
      let c = ACESFilmicToneMapping(vec3(1., 2., 3.), a.get()).mutable();

      for_by(5, |for_ctx, i, _| {
        let b = 1.;
        if_by(i.greater_than(0), || {
          a.set(a.get() + b.into());
          for_ctx.do_continue();
        });

        let r: Node<Vec4<f32>> = (Vec3::zero(), a.get()).into();
        position.set(r);
      });

      if_by(false, || {
        a.set(a.get() + c.get().x());
        let r: Node<Vec4<f32>> = (Vec3::zero(), a.get()).into();
        position.set(r);
      });

      builder.register::<ClipPosition>(position.get());

      Ok(())
    })
  }
}

#[test]
fn test_build_shader() {
  test_provider_success(&Test);
}
