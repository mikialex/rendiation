use crate::*;

pub struct MeshMergeCtx<'a> {
  pub meshes: &'a [&'a MeshEnum],
  pub transforms: &'a [Mat4<f32>],
}

// impl MeshMergeSource for
pub struct MergeImplRegistry {
  implementation: Vec<Box<dyn Fn(&MeshMergeCtx) -> Vec<MeshEnum>>>,
}

impl Default for MergeImplRegistry {
  fn default() -> Self {
    let mut s = Self {
      implementation: Default::default(),
    };
    s.register(merge_attribute_mesh);

    s
  }
}

impl MergeImplRegistry {
  pub fn register(&mut self, f: impl Fn(&MeshMergeCtx) -> Vec<MeshEnum> + 'static) -> usize {
    self.implementation.push(Box::new(f));
    self.implementation.len() - 1
  }
}

fn merge_attribute_mesh(ctx: &MeshMergeCtx) -> Vec<MeshEnum> {
  let sources = ctx.meshes.iter().map(|source| todo!()).collect::<Vec<_>>();
  let world_mats = ctx.transforms.iter();
  let normal_mats = ctx.transforms.iter().map(|mat| mat.to_normal_matrix());
  merge_attributes_meshes(
    &sources,
    |idx, position| position.apply_matrix_into(ctx.transforms[idx]),
    |idx, normal| normal.apply_matrix_into(ctx.transforms[idx].to_normal_matrix().into()),
  )
  .ok();
  todo!()
}

pub const ATTRIBUTE_MERGE: usize = 0;
