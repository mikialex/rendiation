use crate::*;

declare_entity!(ClippingSetEntity);

declare_component!(ClippingSetComponent, ClippingSetEntity, Vec<Plane>);

pub const MAX_CLIPPING_PLANE_SUPPORT_IN_CLIPPING_SET: usize = 8;

pub fn register_clipping_data_model() {
  global_database()
    .declare_entity::<ClippingSetEntity>()
    .declare_component::<ClippingSetComponent>();
}
