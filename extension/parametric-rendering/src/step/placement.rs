use rendiation_step_reader::{
  entities::Axis2Placement3d,
  ruststep::{
    ast::Name,
    tables::{IntoOwned, PlaceHolder},
  },
};

use super::*;
use crate::*;

pub type Placement = (Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>);

pub fn build_assembly_placement_map(
  table: &Table,
) -> std::collections::HashMap<u64, Vec<Placement>> {
  use std::collections::{HashMap, HashSet};
  let srrwt_count = table
    .shape_representation_relationship_with_transformation
    .len();
  if srrwt_count == 0 {
    return HashMap::new();
  }
  crate::step::step_dbg!("step: assembly placement map — srrwt={srrwt_count}");

  // 1. Build SR → breps mapping from ShapeRepresentation items
  let mut sr_to_breps: HashMap<u64, Vec<u64>> = HashMap::new();
  for (sr_id, sr_holder) in &table.shape_representation {
    // Check items for brep references
    let mut breps: Vec<u64> = Vec::new();
    for item_ph in &sr_holder.items {
      if let Some(item_id) = entity_id_from_ph(item_ph) {
        if table.manifold_solid_brep.contains_key(&item_id) {
          breps.push(item_id);
        }
      }
    }
    if !breps.is_empty() {
      sr_to_breps.insert(*sr_id, breps);
    }
  }

  // 2. Build assembly SR → geometry SR links from ShapeRepresentationRelationship.
  // In AP214 assemblies, rep_2 is usually the product/component shape
  // representation, while rep_1 is the representation that owns the BREP.
  let mut representation_children: HashMap<u64, Vec<u64>> = HashMap::new();
  for (_id, srr_holder) in &table.shape_representation_relationship {
    if let (Some(geometry_sr), Some(component_sr)) = (
      get_holder_ref_id(&srr_holder.rep_1),
      get_holder_ref_id(&srr_holder.rep_2),
    ) {
      representation_children
        .entry(component_sr)
        .or_default()
        .push(geometry_sr);
    }
  }

  // 3. Build transform stack from ShapeRepresentationRelationshipWithTransformation.
  // The transform maps rep_1 (child/component coordinates) into rep_2
  // (parent/assembly coordinates), so traversal is rep_2 → rep_1.
  let mut parent_to_children: HashMap<u64, Vec<(u64, Placement)>> = HashMap::new();
  for (_id, srrwt_holder) in &table.shape_representation_relationship_with_transformation {
    let rep1_id = get_holder_ref_id(&srrwt_holder.rep_1);
    let rep2_id = get_holder_ref_id(&srrwt_holder.rep_2);
    let trans_op_id = get_holder_ref_id(&srrwt_holder.transformation_operator);
    let (Some(rep1), Some(rep2), Some(trans_id)) = (rep1_id, rep2_id, trans_op_id) else {
      continue;
    };

    let Some(matrix) = compute_idt_matrix(table, trans_id) else {
      continue;
    };

    parent_to_children
      .entry(rep2)
      .or_default()
      .push((rep1, matrix));
  }

  if parent_to_children.is_empty() {
    return HashMap::new();
  }

  // 4. Walk from assembly roots and keep every occurrence. A single BREP may be
  // instanced many times through the same component representation.
  let all_children: HashSet<u64> = parent_to_children
    .values()
    .flat_map(|v| v.iter().map(|(child, _)| *child))
    .collect();
  let roots: Vec<u64> = parent_to_children
    .keys()
    .filter(|k| !all_children.contains(k))
    .copied()
    .collect();

  let identity = (
    Vec3::zero(),
    Vec3::new(1., 0., 0.),
    Vec3::new(0., 1., 0.),
    Vec3::new(0., 0., 1.),
  );
  let mut brep_placement: HashMap<u64, Vec<Placement>> = HashMap::new();
  for root in &roots {
    let mut stack: Vec<(u64, Placement)> = vec![(*root, identity)];
    while let Some((sr_id, mat)) = stack.pop() {
      if let Some(breps) = sr_to_breps.get(&sr_id) {
        for brep_id in breps {
          brep_placement.entry(*brep_id).or_default().push(mat);
        }
      }

      if let Some(children) = representation_children.get(&sr_id) {
        for child_sr in children {
          stack.push((*child_sr, mat));
        }
      }

      if let Some(children) = parent_to_children.get(&sr_id) {
        for (child_sr, child_mat) in children {
          stack.push((*child_sr, compose_placement(&mat, child_mat)));
        }
      }
    }
  }

  // 5. Report the number of resulting BREP occurrences.
  crate::step::step_dbg!(
    "step: assembly placement — sr_to_breps={} representation_links={} brep_occurrences={}",
    sr_to_breps.len(),
    representation_children.len(),
    brep_placement.values().map(Vec::len).sum::<usize>()
  );
  brep_placement
}

/// Get the entity ID referenced by a PlaceHolder.
fn get_holder_ref_id<T>(ph: &PlaceHolder<T>) -> Option<u64> {
  match ph {
    PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
    _ => None,
  }
}

/// Compute the transform matrix from an ItemDefinedTransformation.
/// Returns the relative placement matrix = mat2 * mat1⁻¹.
fn compute_idt_matrix(table: &Table, idt_entity_id: u64) -> Option<Placement> {
  let idt_holder = table.item_defined_transformation.get(&idt_entity_id)?;
  let ax1_id = get_holder_ref_id(&idt_holder.transform_item_1)?;
  let ax2_id = get_holder_ref_id(&idt_holder.transform_item_2)?;
  let ax1 = table
    .axis2_placement_3d
    .get(&ax1_id)?
    .clone()
    .into_owned(table)
    .ok()?;
  let ax2 = table
    .axis2_placement_3d
    .get(&ax2_id)?
    .clone()
    .into_owned(table)
    .ok()?;
  let mat1 = axis2_placement_to_transform(&ax1);
  let mat2 = axis2_placement_to_transform(&ax2);
  Some(compose_placement(&mat2, &invert_placement(&mat1)))
}

fn invert_placement(pl: &Placement) -> Placement {
  let (origin, x, y, z) = *pl;
  let inv_origin = Vec3::new(-origin.dot(x), -origin.dot(y), -origin.dot(z));
  (
    inv_origin,
    Vec3::new(x.x, y.x, z.x),
    Vec3::new(x.y, y.y, z.y),
    Vec3::new(x.z, y.z, z.z),
  )
}

fn compose_placement(a: &Placement, b: &Placement) -> Placement {
  let (oa, xa, ya, za) = *a;
  let (ob, xb, yb, zb) = *b;
  let origin = oa + xa * ob.x + ya * ob.y + za * ob.z;
  let x = xa * xb.x + ya * xb.y + za * xb.z;
  let y = xa * yb.x + ya * yb.y + za * yb.z;
  let z = xa * zb.x + ya * zb.y + za * zb.z;
  (origin, x, y, z)
}

pub fn build_placement_map(table: &Table) -> std::collections::HashMap<u64, Placement> {
  use std::collections::HashMap;
  let mut map = HashMap::new();
  for (_sr_id, sr) in &table.shape_representation {
    let mut placement: Option<Placement> = None;
    let mut brep_ids: Vec<u64> = Vec::new();
    for item_ph in &sr.items {
      let item_id = match item_ph {
        PlaceHolder::Ref(Name::Entity(id)) => *id,
        _ => continue,
      };
      if let Some(ax_holder) = table.axis2_placement_3d.get(&item_id) {
        if let Ok(ax) = ax_holder.clone().into_owned(table) {
          placement = Some(axis2_placement_to_transform(&ax));
        }
      }
      if table.manifold_solid_brep.contains_key(&item_id) {
        brep_ids.push(item_id);
      }
    }
    if let Some(pl) = placement {
      for bid in brep_ids {
        map.insert(bid, pl);
      }
    }
  }
  map
}

fn axis2_placement_to_transform(ax: &Axis2Placement3d) -> Placement {
  let origin = {
    let coords = &ax.location.coordinates;
    Vec3::new(coords[0] as f32, coords[1] as f32, coords[2] as f32)
  };
  let z = {
    let d = &ax.axis.direction_ratios;
    Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize()
  };
  let x = if let Some(ref ref_dir) = ax.ref_direction {
    let d = &ref_dir.direction_ratios;
    Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize()
  } else {
    let ax = if z.x.abs() > 0.9 {
      Vec3::new(0.0, 1.0, 0.0)
    } else {
      Vec3::new(1.0, 0.0, 0.0)
    };
    z.cross(ax).cross(z).normalize()
  };
  let y = z.cross(x).normalize();
  (origin, x, y, z)
}
