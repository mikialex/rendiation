// use crate::*;

// pub struct AutoInstanceSystem {
//   //
// }

// impl AutoInstanceSystem {
//   pub fn new(
//     scene_delta: impl Stream<Item = SceneInnerDelta>,
//     scene_derived_delta: impl Stream<Item = SceneNodeDerivedDataDelta>,
//   ) -> Self {
//     todo!()
//   }
// }

// impl AutoInstanceSystem {
//   fn transform_scene_delta(&mut self, delta: SceneInnerDelta) -> SceneInnerDelta {
//     match delta {
//       SceneInnerDelta::models(delta) => match delta {
//         arena::ArenaDelta::Mutate(_) => todo!(),
//         arena::ArenaDelta::Insert(_) => todo!(),
//         arena::ArenaDelta::Remove(_) => todo!(),
//       },
//       SceneInnerDelta::nodes(_) => todo!(),
//       _ => delta,
//     }
//   }
// }
