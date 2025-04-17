use crate::*;

declare_entity!(SceneAnimationEntity);
declare_foreign_key!(
  SceneAnimationBelongsToScene,
  SceneAnimationEntity,
  SceneEntity
);

declare_entity!(SceneAnimationChannelEntity);
declare_foreign_key!(
  SceneAnimationChannelTargetNode,
  SceneAnimationChannelEntity,
  SceneNodeEntity
);
declare_foreign_key!(
  SceneAnimationChannelBelongToAnimation,
  SceneAnimationChannelEntity,
  SceneAnimationEntity
);

declare_component!(
  SceneAnimationChannelInterpolation,
  SceneAnimationChannelEntity,
  InterpolationStyle
);

declare_component!(
  SceneAnimationChannelField,
  SceneAnimationChannelEntity,
  SceneAnimationField
);

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Facet)]
pub enum InterpolationStyle {
  #[default]
  Linear,
  Step,
  Cubic,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Facet)]
pub enum SceneAnimationField {
  #[default]
  Position,
  Scale,
  Rotation,
  MorphTargetWeights,
}

declare_entity_associated!(SceneAnimationChannelInput, SceneAnimationChannelEntity);
declare_entity_associated!(SceneAnimationChannelOutput, SceneAnimationChannelEntity);

impl SceneBufferView for SceneAnimationChannelInput {}
impl SceneBufferView for SceneAnimationChannelOutput {}

pub fn register_scene_animation_data_model() {
  global_database()
    .declare_entity::<SceneAnimationEntity>()
    .declare_foreign_key::<SceneAnimationBelongsToScene>();

  let ecg = global_database()
    .declare_entity::<SceneAnimationChannelEntity>()
    .declare_foreign_key::<SceneAnimationChannelBelongToAnimation>()
    .declare_foreign_key::<SceneAnimationChannelTargetNode>()
    .declare_component::<SceneAnimationChannelInterpolation>()
    .declare_component::<SceneAnimationChannelField>();

  let ecg = register_scene_buffer_view::<SceneAnimationChannelInput>(ecg);
  let _ = register_scene_buffer_view::<SceneAnimationChannelOutput>(ecg);
}

pub struct AnimationChannelDataView {
  pub sampler: AnimationSampler,
  pub target_node: EntityHandle<SceneNodeEntity>,
  pub animation: EntityHandle<SceneAnimationEntity>,
}

impl AnimationChannelDataView {
  pub fn write(&self, writer: &mut SceneWriter) -> EntityHandle<SceneAnimationChannelEntity> {
    writer
      .animation_channel
      .component_value_writer::<SceneAnimationChannelField>(self.sampler.field)
      .component_value_writer::<SceneAnimationChannelInterpolation>(self.sampler.interpolation)
      .component_value_writer::<SceneAnimationChannelTargetNode>(self.target_node.some_handle())
      .component_value_writer::<SceneAnimationChannelBelongToAnimation>(
        self.animation.some_handle(),
      );

    let data = self.sampler.input.clone().write(&mut writer.buffer_writer);
    let input = SceneBufferViewDataView {
      data: Some(data),
      range: None,
      count: Some(self.sampler.input.count as u32),
    };
    let data = self.sampler.output.clone().write(&mut writer.buffer_writer);
    let output = SceneBufferViewDataView {
      data: Some(data),
      range: None,
      count: Some(self.sampler.output.count as u32),
    };

    input.write::<SceneAnimationChannelInput>(&mut writer.animation_channel);
    output.write::<SceneAnimationChannelOutput>(&mut writer.animation_channel);

    writer.animation_channel.new_entity()
  }
}

/// An animation sampler combines timestamps with a sequence of
/// output values and defines an interpolation algorithm.
#[derive(Clone)]
pub struct AnimationSampler {
  pub interpolation: InterpolationStyle,
  pub field: SceneAnimationField,
  pub input: AttributeAccessor,
  pub output: AttributeAccessor,
}

impl AnimationSampler {
  pub fn sample_animation(&self, time: f32) -> Option<InterpolationItem> {
    let sample_time = self.get_looped_sample_time(time);
    let (mut spline, (start_time, end_time)) =
      InterpolateInstance::try_from_sampler(self, sample_time)?;
    let normalized_time = (end_time - sample_time) / (end_time - start_time);
    spline.sample_animation(normalized_time)
  }

  pub fn get_start_end_time(&self) -> (f32, f32) {
    let start = self.input.read().get::<f32>(0).unwrap();
    let end = self.input.read().get::<f32>(self.input.count - 1).unwrap();
    (start, end)
  }

  pub fn get_looped_sample_time(&self, abs_time: f32) -> f32 {
    let (start_time, end_time) = self.get_start_end_time();
    let length = end_time - start_time;
    let remind = abs_time - (abs_time / length).floor() * length;
    start_time + remind
  }
}

/// this is an optimization, based on the hypnosis that the interpolation spline
/// will be reused in next sample, which avoid the slow underlayer sampler retrieving
pub struct AnimationSamplerExecutor {
  spline: Option<(InterpolateInstance<InterpolationItem>, (f32, f32))>,
  sampler: AnimationSampler,
}

impl AnimationSamplerExecutor {
  pub fn sample_animation(&mut self, time: f32) -> Option<InterpolationItem> {
    loop {
      // do we have get_or_insert_with_option?
      if let Some((spline, (start_time, end_time))) = &mut self.spline {
        let normalized_time = (*end_time - time) / (*end_time - *start_time);
        if 0. < normalized_time && normalized_time <= 1.0 {
          break spline.sample_animation(normalized_time);
        } else {
          self.spline = None;
        }
      } else {
        self.spline =
          InterpolateInstance::<InterpolationItem>::try_from_sampler(&self.sampler, time)?.into();
      }
    }
  }
}

#[derive(Copy, Clone)]
pub enum InterpolationItem {
  Position(Vec3<f32>),
  Scale(Vec3<f32>),
  Quaternion(Quat<f32>),
  MorphTargetWeights(f32),
}

impl InterpolationItem {
  fn interpolate(self, other: Self, t: f32) -> Option<Self> {
    use InterpolationItem::*;
    match (self, other) {
      (Position(a), Position(b)) => Position(a.lerp(b, t)),
      (Scale(a), Scale(b)) => Scale(a.lerp(b, t)),
      (Quaternion(a), Quaternion(b)) => Quaternion(a.slerp(b, t)),
      (MorphTargetWeights(a), MorphTargetWeights(b)) => MorphTargetWeights(a.lerp(b, t)),
      _ => return None,
    }
    .into()
  }
}

#[derive(Copy, Clone)]
enum InterpolationCubicItem {
  Position(CubicVertex<Vec3<f32>>),
  Scale(CubicVertex<Vec3<f32>>),
  Quaternion(CubicVertex<Quat<f32>>),
  MorphTargetWeights(CubicVertex<f32>),
}

impl InterpolationCubicItem {
  pub fn transpose(self) -> CubicVertex<InterpolationItem> {
    macro_rules! cubic {
      ($v: tt, $variant: tt) => {
        CubicVertex {
          enter: InterpolationItem::$variant($v.enter),
          center: InterpolationItem::$variant($v.center),
          exit: InterpolationItem::$variant($v.exit),
        }
      };
    }
    match self {
      InterpolationCubicItem::Position(v) => cubic!(v, Position),
      InterpolationCubicItem::Scale(v) => cubic!(v, Scale),
      InterpolationCubicItem::Quaternion(v) => cubic!(v, Quaternion),
      InterpolationCubicItem::MorphTargetWeights(v) => cubic!(v, MorphTargetWeights),
    }
  }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CubicVertex<T> {
  enter: T,
  center: T,
  exit: T,
}
unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for CubicVertex<T> {}
unsafe impl<T: bytemuck::Pod> bytemuck::Pod for CubicVertex<T> {}

enum InterpolateInstance<T> {
  Linear {
    start: T,
    end: T,
  },
  Step {
    start: T,
    end: T,
  },
  Cubic {
    start: T,
    ctrl1: T,
    ctrl2: T,
    end: T,
  },
}

impl InterpolateInstance<InterpolationItem> {
  fn sample_animation(&mut self, t: f32) -> Option<InterpolationItem> {
    match self {
      InterpolateInstance::Step { start, end } => if t == 1. { *end } else { *start }.into(),
      InterpolateInstance::Linear { start, end } => start.interpolate(*end, t),
      InterpolateInstance::Cubic {
        start,
        ctrl1,
        ctrl2,
        end,
      } => {
        let t1 = start.interpolate(*ctrl1, t)?;
        let t2 = ctrl1.interpolate(*ctrl2, t)?;
        let t3 = ctrl2.interpolate(*end, t)?;

        let t4 = t1.interpolate(t2, t)?;
        let t5 = t2.interpolate(t3, t)?;

        t4.interpolate(t5, t)
      }
    }
  }
}

impl InterpolateInstance<InterpolationItem> {
  fn try_from_sampler(sampler: &AnimationSampler, time: f32) -> Option<(Self, (f32, f32))> {
    // decide which frame interval we are in;
    let sampler_input = sampler.input.read();
    let slice = sampler_input.visit_slice::<f32>()?;

    // the gltf animation spec doesn't contains start time or loop behavior, we just use abs time
    let end_index = slice
      .binary_search_by(|v| v.partial_cmp(&time).unwrap_or(core::cmp::Ordering::Equal))
      .unwrap_or_else(|e| e);
    let len = slice.len();

    // time is out of sampler range
    if end_index == 0 || end_index == len {
      return None;
    }

    let (start_time, start_index) = (sampler_input.get::<f32>(end_index - 1)?, end_index - 1);
    let (end_time, end_index) = (sampler_input.get::<f32>(end_index)?, end_index);
    let field_ty = sampler.field;

    fn get_output_single(
      output: &AttributeAccessor,
      index: usize,
      field_ty: SceneAnimationField,
    ) -> Option<InterpolationItem> {
      use SceneAnimationField::*;
      let output = output.read();
      match field_ty {
        MorphTargetWeights => InterpolationItem::MorphTargetWeights(output.get::<f32>(index)?),
        Position => InterpolationItem::Position(output.get::<Vec3<f32>>(index)?),
        Rotation => InterpolationItem::Quaternion(output.get::<Quat<f32>>(index)?),
        Scale => InterpolationItem::Scale(output.get::<Vec3<f32>>(index)?),
      }
      .into()
    }

    fn get_output_cubic(
      output: &AttributeAccessor,
      index: usize,
      field_ty: SceneAnimationField,
    ) -> Option<InterpolationCubicItem> {
      use InterpolationCubicItem::*;
      use SceneAnimationField as SF;
      let output = output.read();
      match field_ty {
        SF::MorphTargetWeights => MorphTargetWeights(output.get::<CubicVertex<f32>>(index)?),
        SF::Position => Position(output.get::<CubicVertex<Vec3<f32>>>(index)?),
        SF::Rotation => Quaternion(output.get::<CubicVertex<Quat<f32>>>(index)?),
        SF::Scale => Scale(output.get::<CubicVertex<Vec3<f32>>>(index)?),
      }
      .into()
    }

    let curve = match sampler.interpolation {
      InterpolationStyle::Linear => InterpolateInstance::Linear {
        start: get_output_single(&sampler.output, start_index, field_ty)?,
        end: get_output_single(&sampler.output, end_index, field_ty)?,
      },
      InterpolationStyle::Step => InterpolateInstance::Step {
        start: get_output_single(&sampler.output, start_index, field_ty)?,
        end: get_output_single(&sampler.output, end_index, field_ty)?,
      },
      InterpolationStyle::Cubic => {
        let cubic_vertex_a = get_output_cubic(&sampler.output, start_index, field_ty)?.transpose();
        let cubic_vertex_b = get_output_cubic(&sampler.output, end_index, field_ty)?.transpose();
        InterpolateInstance::Cubic {
          start: cubic_vertex_a.center,
          ctrl1: cubic_vertex_a.exit,
          ctrl2: cubic_vertex_b.enter,
          end: cubic_vertex_b.center,
        }
      }
    };

    (curve, (start_time, end_time)).into()
  }
}
