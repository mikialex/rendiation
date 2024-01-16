mod states;
pub use states::*;
mod flat;
pub use flat::*;
mod physical_sg;
pub use physical_sg::*;
mod physical_mr;
pub use physical_mr::*;
mod utils;
pub use utils::*;

use crate::*;

fn tex_sample_handle_of_material<M>(
  checker: impl Fn(DeltaOf<M>) -> Option<AllocIdx<SceneTexture2DType>>,
  texture2ds: impl ReactiveCollection<AllocIdx<SceneTexture2DType>, TextureSamplerHandlePair>,
) -> impl ReactiveCollection<AllocIdx<M>, TextureSamplerHandlePair> {
  storage_of::<M>().listen_to_reactive_collection(|_| Some(()))
}
