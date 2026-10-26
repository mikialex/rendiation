use crate::*;

mod attribute;
pub use attribute::*;

pub struct IndicesBufferInfo {
  pub buffer: AbstractReadonlyStorageBuffer<[u32]>,
  pub should_access_as_u16: bool,
}

pub type ResolvedIndirectModelShapeRendererBox<'a> =
  Box<dyn ResolvedIndirectModelShapeRenderer + 'a, &'a dyn Allocator>;

/// Resolve the indirect shape renderer of a standard model for one kind of shape (attribute
/// mesh, cell mesh, ...).
///
/// A list of resolvers is also a resolver, it selects the first one responsible for the model.
pub trait IndirectModelShapeRendererResolver {
  /// see [IndirectModelRendererResolver::resolve]
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelShapeRendererBox<'a>, SceneModelRenderDataError>>;
}

impl IndirectModelShapeRendererResolver for [Box<dyn IndirectModelShapeRendererResolver>] {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelShapeRendererBox<'a>, SceneModelRenderDataError>> {
    self
      .iter()
      .find_map(|resolver| resolver.resolve(idx, alloc))
  }
}

impl IndirectModelShapeRendererResolver for Vec<Box<dyn IndirectModelShapeRendererResolver>> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<StandardModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedIndirectModelShapeRendererBox<'a>, SceneModelRenderDataError>> {
    self.as_slice().resolve(idx, alloc)
  }
}

/// The indirect shape renderer resolved for one standard model.
pub trait ResolvedIndirectModelShapeRenderer {
  /// see [ResolvedIndirectModelRenderer::resolver_type_id]
  fn resolver_type_id(&self) -> TypeId;

  fn hash_shader_group_key(&self, hasher: &mut PipelineHasher);
  fn hash_shader_group_key_with_self_type_info(&self, hasher: &mut PipelineHasher) {
    self.hash_shader_group_key(hasher);
    hasher.hash(self.resolver_type_id());
  }

  /// see [ResolvedIndirectModelRenderer::impl_distinguish_key], the key must contain the type
  /// identity of the impl.
  fn impl_distinguish_key(&self) -> u64;

  /// see [ResolvedIndirectModelRenderer::use_create_or_update_indirect_draw_providers]
  fn use_create_or_update_indirect_draw_providers(
    &self,
    cx: &mut DeviceParallelComputeCtx,
    list: &DeviceDrawList,
  ) -> Result<Vec<Box<dyn IndirectDrawProvider>>, SceneModelRenderDataError>;

  fn make_draw_command_builder(&self) -> Result<DrawCommandBuilder, SceneModelRenderDataError>;

  fn make_component_indirect<'a>(&'a self, alloc: &'a dyn Allocator) -> RenderComponentBox<'a>;

  /// If the indirect shape implementation using index buffer, then it's buffer must also has
  /// storage usage and can be accessed here. This api is to expose the ability to convert the
  /// draw index call into draw array in some case.
  ///
  /// Return None if the shape is not indexed
  fn get_index_storage_buffer(&self) -> Option<IndicesBufferInfo>;
}
