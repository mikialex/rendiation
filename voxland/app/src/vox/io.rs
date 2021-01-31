use tokio::fs::File;
use tokio::prelude::*;
use super::{block_coords::ChunkCoords, chunk::Chunk};
use std::collections::HashSet;

pub struct WorldIOManager{
  chunks_to_save: HashSet<Chunk>,
  chunks_to_load: HashSet<Chunk>
}

impl WorldIOManager{
  pub fn new() -> Self {
    Self{
      chunks_to_save: HashSet::new(),
      chunks_to_load: HashSet::new()
    }
  }

  pub async fn page_out_chunk(chunk: Chunk){
    todo!()
  }

  pub async fn page_in_chunk(chunk_key: ChunkCoords) -> Chunk{
    todo!()
  }
}
