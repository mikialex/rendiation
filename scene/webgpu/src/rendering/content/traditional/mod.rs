use crate::*;

mod model;
pub use model::*;

mod list;
pub use list::*;

// impl SceneRenderable for SceneModel {
//   fn render(
//     &self,
//     pass: &mut FrameRenderPass,
//     dispatcher: &dyn RenderComponentAny,
//     camera: &SceneCamera,
//     scene: &SceneRenderResourceGroup,
//   ) {
//     self.visit(|model| model.render(pass, dispatcher, camera, scene))
//   }
// }

// impl SceneRenderable for SceneModelImpl {
//   fn render(
//     &self,
//     pass: &mut FrameRenderPass,
//     dispatcher: &dyn RenderComponentAny,
//     camera: &SceneCamera,
//     scene: &SceneRenderResourceGroup,
//   ) {
//     setup_pass_core(self, pass, camera, None, dispatcher, scene);
//   }
// }
