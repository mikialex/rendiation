use incremental::*;

use crate::*;

// todo support generics latter
clone_self_incremental!(Vec2<f32>);
clone_self_incremental!(Vec3<f32>);
clone_self_incremental!(Vec4<f32>);

clone_self_incremental!(Mat2<f32>);
clone_self_incremental!(Mat3<f32>);
clone_self_incremental!(Mat4<f32>);
