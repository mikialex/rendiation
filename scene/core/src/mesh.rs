use futures::StreamExt;
use reactive::once_forever_pending;
use reactive::{PollUtils, SignalStreamExt};
use rendiation_geometry::{Box3, Ray3};
use rendiation_geometry::{OptionalNearest, SpaceBounding};
use rendiation_mesh_core::*;

use crate::*;

#[derive(Clone)]
pub enum MeshEnum {
  AttributesMesh(IncrementalSignalPtr<AttributesMesh>),
  TransformInstanced(IncrementalSignalPtr<TransformInstancedSceneMesh>),
  Foreign(ForeignObject),
}

clone_self_incremental!(MeshEnum);

pub fn register_core_mesh_features<T>()
where
  T: AsRef<dyn IntersectAbleGroupedMesh>
    + AsMut<dyn IntersectAbleGroupedMesh>
    + AsRef<dyn GlobalIdentified>
    + AsMut<dyn GlobalIdentified>
    // + AsRef<dyn WatchableSceneMeshLocalBounding>
    // + AsMut<dyn WatchableSceneMeshLocalBounding>
    + 'static,
{
  get_dyn_trait_downcaster_static!(GlobalIdentified).register::<T>();
  get_dyn_trait_downcaster_static!(IntersectAbleGroupedMesh).register::<T>();
}

impl MeshEnum {
  pub fn guid(&self) -> Option<u64> {
    match self {
      Self::AttributesMesh(m) => m.guid(),
      Self::TransformInstanced(m) => m.guid(),
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(GlobalIdentified)
        .downcast_ref(m.as_ref().as_any())?
        .guid(),
    }
    .into()
  }
}

#[derive(Clone)]
pub struct TransformInstancedSceneMesh {
  pub mesh: MeshEnum,
  pub transforms: Vec<Mat4<f32>>,
}
clone_self_incremental!(TransformInstancedSceneMesh);

pub trait WatchableSceneMeshLocalBounding {
  fn build_local_bound_stream(&self) -> Box<dyn Stream<Item = Option<Box3>> + Unpin>;
}
define_dyn_trait_downcaster_static!(WatchableSceneMeshLocalBounding);

impl WatchableSceneMeshLocalBounding for MeshEnum {
  fn build_local_bound_stream(&self) -> Box<dyn Stream<Item = Option<Box3>> + Unpin> {
    match self {
      MeshEnum::AttributesMesh(mesh) => {
        let st = mesh
          .single_listen_by(any_change)
          .filter_map_sync(mesh.defer_weak())
          .map(|mesh| {
            let mesh = mesh.read();
            let local: Box3 = mesh
              .read_shape()
              .primitive_iter()
              .map(|p| p.to_bounding())
              .collect();
            local.into()
          });
        Box::new(st) as Box<dyn Stream<Item = Option<Box3>> + Unpin>
      }
      MeshEnum::TransformInstanced(mesh) => {
        let st = mesh
          .single_listen_by(any_change)
          .filter_map_sync(mesh.defer_weak())
          .map(|mesh| {
            let mesh = mesh.read();

            let inner_bounding = mesh
              .mesh
              .build_local_bound_stream()
              .consume_self_get_next()
              .unwrap();

            inner_bounding.map(|inner_bounding| {
              mesh
                .transforms
                .iter()
                .map(|mat| inner_bounding.apply_matrix_into(*mat))
                .collect::<Box3>()
            })
          });
        Box::new(st)
      }
      MeshEnum::Foreign(mesh) => {
        if let Some(mesh) = get_dyn_trait_downcaster_static!(WatchableSceneMeshLocalBounding)
          .downcast_ref(mesh.as_ref().as_any())
        {
          mesh.build_local_bound_stream()
        } else {
          Box::new(once_forever_pending(None))
        }
      }
    }
  }
}

impl IntersectAbleGroupedMesh for TransformInstancedSceneMesh {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    self.transforms.iter().for_each(|mat| {
      let world_inv = mat.inverse_or_identity();
      let local_ray = ray.clone().apply_matrix_into(world_inv);
      self
        .mesh
        .intersect_list_by_group(local_ray, conf, result, group)
    })
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    self
      .transforms
      .iter()
      .fold(OptionalNearest::none(), |mut pre, mat| {
        let world_inv = mat.inverse_or_identity();
        let local_ray = ray.clone().apply_matrix_into(world_inv);
        let r = self.mesh.intersect_nearest_by_group(local_ray, conf, group);
        *pre.refresh_nearest(r)
      })
  }
}

impl IntersectAbleGroupedMesh for MeshEnum {
  fn intersect_list_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    result: &mut MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    match self {
      MeshEnum::AttributesMesh(mesh) => mesh
        .read()
        .read_shape()
        .intersect_list_by_group(ray, conf, result, group),
      MeshEnum::TransformInstanced(mesh) => mesh
        .read()
        .intersect_list_by_group(ray, conf, result, group),
      MeshEnum::Foreign(mesh) => {
        if let Some(pickable) = get_dyn_trait_downcaster_static!(IntersectAbleGroupedMesh)
          .downcast_ref(mesh.as_ref().as_any())
        {
          pickable.intersect_list_by_group(ray, conf, result, group)
        }
      }
    }
  }

  fn intersect_nearest_by_group(
    &self,
    ray: Ray3,
    conf: &MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    match self {
      MeshEnum::AttributesMesh(mesh) => mesh
        .read()
        .read_shape()
        .intersect_nearest_by_group(ray, conf, group),
      MeshEnum::TransformInstanced(mesh) => {
        mesh.read().intersect_nearest_by_group(ray, conf, group)
      }
      MeshEnum::Foreign(mesh) => {
        if let Some(pickable) = get_dyn_trait_downcaster_static!(IntersectAbleGroupedMesh)
          .downcast_ref(mesh.as_ref().as_any())
        {
          pickable.intersect_nearest_by_group(ray, conf, group)
        } else {
          OptionalNearest::none()
        }
      }
    }
  }
}
