use super::*;
use crate::*;

#[derive(Clone)]
pub struct InstanceDrawCommandBuilder {
  pub internal: Box<dyn NoneIndexedDrawCommandBuilder>,
  pub instance_meta: AbstractReadonlyStorageBuffer<[InstanceMetaData]>,
  pub model_to_instance: AbstractReadonlyStorageBuffer<[u32]>,
  pub model_to_instance_host: ForeignKeyReadView<SceneModelTransformInstancedModelPayload>,
  pub source_model_host: ForeignKeyReadView<TransformInstancedModelRefSceneModel>,
  pub instance_meta_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<InstanceMetaData>>,
}

impl ShaderHashProvider for InstanceDrawCommandBuilder {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.internal.hash_pipeline_with_type_info(hasher);
  }
}

struct InstanceNoneIndexedInvocation {
  inner: Box<dyn NoneIndexedDrawCommandBuilderInvocation>,
  instance_meta: ShaderReadonlyPtrOf<[InstanceMetaData]>,
  model_to_instance: ShaderReadonlyPtrOf<[u32]>,
}

impl NoneIndexedDrawCommandBuilderInvocation for InstanceNoneIndexedInvocation {
  fn generate_draw_command(&self, draw_id: Node<u32>) -> Node<DrawIndirectArgsStorage> {
    let instance_model_id = self.model_to_instance.index(draw_id).load();
    let source_model_id = self
      .instance_meta
      .index(instance_model_id)
      .origin_model()
      .load();
    let inner = self.inner.generate_draw_command(source_model_id).expand();
    let instance_count = self
      .instance_meta
      .index(instance_model_id)
      .instance_count()
      .load();

    ENode::<DrawIndirectArgsStorage> {
      vertex_count: inner.vertex_count * instance_count,
      instance_count: inner.instance_count,
      base_vertex: inner.base_vertex,
      base_instance: draw_id,
    }
    .construct()
  }
}

impl NoneIndexedDrawCommandBuilder for InstanceDrawCommandBuilder {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let instance_model_id = self.model_to_instance_host.get(id)?;
    let source_model = self.source_model_host.get(instance_model_id)?;
    let internal_draw = self.internal.draw_command_host_access(source_model)?;
    let instance_count = self
      .instance_meta_host
      .get(instance_model_id.alloc_index())?
      .instance_count;

    match internal_draw {
      DrawCommand::Array {
        vertices,
        instances,
      } => {
        assert_eq!(instances, 0..1);
        let count = vertices.end - vertices.start;
        let new_count = count * instance_count;
        let vertices = vertices.start..(vertices.start + new_count);
        DrawCommand::Array {
          vertices,
          instances: 0..1,
        }
      }
      DrawCommand::Indexed { .. } => assert_none_indexed(),
      _ => unreachable!("unexpected draw command type"),
    }
    .into()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let instance_meta = cx.bind_by(&self.instance_meta);
    let model_to_instance = cx.bind_by(&self.model_to_instance);

    let inner = self.internal.build_invocation(cx);
    Box::new(InstanceNoneIndexedInvocation {
      inner,
      instance_meta,
      model_to_instance,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.instance_meta);
    builder.bind(&self.model_to_instance);
    self.internal.bind(builder);
  }
}
