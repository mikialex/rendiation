//! Assembly structure (AP203 / AP214) to body occurrences with world transforms.
//!
//! Representations are the nodes. Three kinds of links exist between them:
//!
//! - A `SHAPE_REPRESENTATION_RELATIONSHIP` without transformation that is not used by a
//!   `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` puts both representations in the same coordinate
//!   space (typically a product `SHAPE_REPRESENTATION` and its `ADVANCED_BREP_SHAPE_REPRESENTATION`).
//!   A "space" is the closure of representations under these links.
//! - A relationship used by a `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` places the child product
//!   of a `NEXT_ASSEMBLY_USAGE_OCCURRENCE` (`related_product_definition`) into its parent
//!   (`relating_product_definition`). The NAUO decides which of `rep_1` / `rep_2` is the child,
//!   by matching them against the representations of both product definitions (found through
//!   `SHAPE_DEFINITION_REPRESENTATION`). A `SHAPE_REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION`
//!   not used by any CDSR is taken as `rep_1` placed into `rep_2`.
//! - `MAPPED_ITEM` in a representation places `REPRESENTATION_MAP.mapped_representation`.
//!
//! Transform convention (ISO 10303-43 `item_defined_transformation`, same as OCCT
//! `STEPControl_ActorRead::ComputeTransformation`): the transformation maps `transform_item_1`
//! (a placement in `rep_1`) onto `transform_item_2` (a placement in `rep_2`), so coordinates of
//! `rep_1` map into `rep_2` with `T = M(item_2) · M(item_1)⁻¹`, where `M(a)` is the local to
//! parent matrix of placement `a`. When the NAUO shows that `rep_2` is the child, the inverse
//! `M(item_1) · M(item_2)⁻¹` places the child into the parent. A mapped item uses
//! `M(mapping_target) · M(mapping_origin)⁻¹`. World transforms compose from the roots
//! (product definitions that are never a NAUO child): `world(child) = world(parent) · T`.
//!
//! The `AXIS2_PLACEMENT_3D` items of a shape representation only define reference placements
//! for these relationships, they are never applied to the geometry by themselves.

use fast_hash_collection::*;
use rendiation_step_reader::ruststep::ast::Parameter;

use super::builder::*;
use super::lookup::*;
use super::*;

pub(super) struct RepInfo {
  pub items: Vec<u64>,
  pub context: Option<u64>,
}

struct ChildLink {
  child: u64,
  /// Extra representations of the child space (the child product definition's own
  /// representations when the link comes from a NAUO).
  extra: Vec<u64>,
  transform: Mat4<f64>,
  label: String,
}

/// Safety limit against pathological instance explosions.
const MAX_OCCURRENCES: usize = 1_000_000;
const MAX_DEPTH: usize = 64;

impl Builder<'_> {
  /// All representations with their items: typed shape representations and representations,
  /// plus untyped entities whose record looks like `XXX_REPRESENTATION(name, (items), context)`.
  pub fn representations(&self) -> FastHashMap<u64, RepInfo> {
    let t = self.t;
    let mut reps = FastHashMap::default();
    for (id, r) in &t.shape_representation {
      reps.insert(
        *id,
        RepInfo {
          items: r.items.iter().filter_map(ref_id).collect(),
          context: ref_id(&r.context_of_items),
        },
      );
    }
    for (id, r) in &t.representation {
      reps.insert(
        *id,
        RepInfo {
          items: r.items.iter().filter_map(ref_id).collect(),
          context: ref_id(&r.context_of_items),
        },
      );
    }
    for (id, records) in &t.unrecognized_records {
      let rep = records.iter().find_map(|r| {
        let is_rep = r.name == "REPRESENTATION" || r.name.ends_with("_REPRESENTATION");
        match &r.parameter {
          Parameter::List(p) if is_rep && p.len() == 3 => match (&p[1], &p[2]) {
            (Parameter::List(_), Parameter::Ref(_)) => Some(RepInfo {
              items: param_refs(&p[1]),
              context: param_ref(&p[2]),
            }),
            _ => None,
          },
          _ => None,
        }
      });
      if let Some(rep) = rep {
        reps.insert(*id, rep);
      }
    }
    reps
  }

  pub fn resolve_occurrences(&mut self) {
    let t = self.t;
    let reps = self.representations();

    // Product definition -> its representations (through SHAPE_DEFINITION_REPRESENTATION).
    let mut pd_reps: FastHashMap<u64, Vec<u64>> = FastHashMap::default();
    for sdr in t.shape_definition_representation.values() {
      let pd = ref_id(&sdr.definition)
        .and_then(|pds| t.product_definition_shape.get(&pds))
        .and_then(|pds| ref_id(&pds.definition))
        .filter(|pd| t.product_definition.contains_key(pd));
      if let (Some(pd), Some(rep)) = (pd, ref_id(&sdr.used_representation)) {
        pd_reps.entry(pd).or_default().push(rep);
      }
    }
    for list in pd_reps.values_mut() {
      list.sort_unstable();
      list.dedup();
    }

    // NAUO: (relating = parent, related = child).
    let mut nauos = FastHashMap::default();
    for id in sorted_ids(&t.next_assembly_usage_occurrence) {
      let n = &t.next_assembly_usage_occurrence[&id];
      match (
        ref_id(&n.relating_product_definition),
        ref_id(&n.related_product_definition),
      ) {
        (Some(parent), Some(child)) => {
          nauos.insert(id, (parent, child));
        }
        _ => self.warn(format!(
          "NAUO #{id}: inline product definition not supported"
        )),
      }
    }

    // Relationship -> the NAUO it places (through CONTEXT_DEPENDENT_SHAPE_REPRESENTATION).
    let mut rel_nauo = FastHashMap::default();
    for id in sorted_ids(&t.context_dependent_shape_representation) {
      let cdsr = &t.context_dependent_shape_representation[&id];
      let rel = ref_id(&cdsr.representation_relation);
      let nauo = ref_id(&cdsr.represented_product_relation)
        .and_then(|pds| t.product_definition_shape.get(&pds))
        .and_then(|pds| ref_id(&pds.definition))
        .filter(|n| nauos.contains_key(n));
      match (rel, nauo) {
        (Some(rel), Some(nauo)) => {
          if rel_nauo.insert(rel, nauo).is_some() {
            self.warn(format!("relationship #{rel} is used by several CDSR"));
          }
        }
        _ => self.warn(format!(
          "CDSR #{id}: relationship or NAUO can not be resolved"
        )),
      }
    }

    // (id, rep_1, rep_2, transformation operator)
    let mut rels = Vec::new();
    for id in sorted_ids(&t.shape_representation_relationship) {
      let r = &t.shape_representation_relationship[&id];
      if let (Some(r1), Some(r2)) = (ref_id(&r.rep_1), ref_id(&r.rep_2)) {
        rels.push((id, r1, r2, None));
      }
    }
    for id in sorted_ids(&t.shape_representation_relationship_with_transformation) {
      let r = &t.shape_representation_relationship_with_transformation[&id];
      if let (Some(r1), Some(r2)) = (ref_id(&r.rep_1), ref_id(&r.rep_2)) {
        rels.push((id, r1, r2, Some(ref_id(&r.transformation_operator))));
      }
    }

    let mut same_space: FastHashMap<u64, Vec<u64>> = FastHashMap::default();
    for (id, r1, r2, op) in &rels {
      if op.is_none() && !rel_nauo.contains_key(id) {
        same_space.entry(*r1).or_default().push(*r2);
        same_space.entry(*r2).or_default().push(*r1);
      }
    }
    // Sorted representation ids of the space containing `seed`.
    let closure = |seed: &[u64]| -> Vec<u64> {
      let mut set: FastHashSet<u64> = seed.iter().copied().collect();
      let mut stack = seed.to_vec();
      while let Some(r) = stack.pop() {
        for n in same_space.get(&r).into_iter().flatten() {
          if set.insert(*n) {
            stack.push(*n);
          }
        }
      }
      let mut space: Vec<_> = set.into_iter().collect();
      space.sort_unstable();
      space
    };
    let no_reps = Vec::new();
    let pd_space = |pd: u64| closure(pd_reps.get(&pd).unwrap_or(&no_reps));

    let mut links: FastHashMap<u64, Vec<ChildLink>> = FastHashMap::default();
    for (id, r1, r2, op) in rels {
      let nauo = rel_nauo.get(&id).copied();
      if op.is_none() && nauo.is_none() {
        continue;
      }
      // `rep_1` is the child unless the NAUO says otherwise.
      let mut reversed = false;
      let mut extra = Vec::new();
      if let Some(n) = nauo {
        let (parent, child) = nauos[&n];
        let (sp, sc) = (pd_space(parent), pd_space(child));
        let standard = sc.binary_search(&r1).is_ok() || sp.binary_search(&r2).is_ok();
        let reverse = sc.binary_search(&r2).is_ok() || sp.binary_search(&r1).is_ok();
        match (standard, reverse) {
          (true, false) => {}
          (false, true) => reversed = true,
          _ => self.warn(format!(
            "relationship #{id}: NAUO #{n} does not tell which representation is the child, assuming rep_1"
          )),
        }
        extra = pd_reps.get(&child).cloned().unwrap_or_default();
      }
      let transform = match op {
        None => Mat4::identity(),
        Some(op) => match op
          .ok_or_else(|| "inline transformation".to_string())
          .and_then(|op| self.item_defined_transformation(op))
        {
          Ok((f1, f2)) if !reversed => f2.to_mat4() * f1.to_mat4_inverse(),
          Ok((f1, f2)) => f1.to_mat4() * f2.to_mat4_inverse(),
          Err(e) => {
            self.warn(format!("relationship #{id} skipped: {e}"));
            continue;
          }
        },
      };
      let (parent_rep, child_rep) = if reversed { (r1, r2) } else { (r2, r1) };
      links.entry(parent_rep).or_default().push(ChildLink {
        child: child_rep,
        extra,
        transform,
        label: nauo.map_or(format!("#{id}"), |n| format!("#{id}(NAUO #{n})")),
      });
    }

    let mut rep_ids: Vec<_> = reps.keys().copied().collect();
    rep_ids.sort_unstable();
    for rep_id in rep_ids {
      for item in &reps[&rep_id].items {
        let Some(mi) = t.mapped_item.get(item) else {
          continue;
        };
        match self.mapped_item(ref_id(&mi.mapping_source), ref_id(&mi.mapping_target)) {
          Ok((mapped, transform)) => links.entry(rep_id).or_default().push(ChildLink {
            child: mapped,
            extra: Vec::new(),
            transform,
            label: format!("#{item}(MAPPED_ITEM)"),
          }),
          Err(e) => self.warn(format!("mapped item #{item} skipped: {e}")),
        }
      }
    }

    // Roots: product definitions that are never a NAUO child.
    let children: FastHashSet<u64> = nauos.values().map(|(_, c)| *c).collect();
    let mut roots: Vec<_> = pd_reps
      .keys()
      .copied()
      .filter(|pd| !children.contains(pd))
      .collect();
    roots.sort_unstable();
    let mut stack: Vec<(Vec<u64>, Mat4<f64>, String, Vec<u64>)> = roots
      .iter()
      .rev()
      .map(|pd| {
        (
          pd_space(*pd),
          Mat4::identity(),
          format!("#{pd}"),
          Vec::new(),
        )
      })
      .collect();

    let mut reached = vec![false; self.m.bodies.len()];
    let mut truncated = false;
    while let Some((space, world, path, ancestors)) = stack.pop() {
      let mut bodies: Vec<_> = space
        .iter()
        .filter_map(|r| reps.get(r))
        .flat_map(|rep| rep.items.iter())
        .filter_map(|i| self.bodies.get(i).copied())
        .collect();
      bodies.sort_unstable();
      bodies.dedup();
      for body in bodies {
        if self.m.occurrences.len() >= MAX_OCCURRENCES {
          truncated = true;
          break;
        }
        reached[body.index()] = true;
        self.m.occurrences.push(Occurrence {
          body,
          transform: world,
          path: path.clone(),
        });
      }
      if truncated {
        break;
      }
      let mut ancestors = ancestors;
      ancestors.extend(space.iter().copied());
      // Push in reverse so that children are visited in link order.
      let child_links: Vec<_> = space
        .iter()
        .flat_map(|r| links.get(r).into_iter().flatten())
        .collect();
      for link in child_links.into_iter().rev() {
        let mut seed = vec![link.child];
        seed.extend(&link.extra);
        let child_space = closure(&seed);
        if child_space.iter().any(|r| ancestors.contains(r))
          || path.matches('>').count() >= MAX_DEPTH
        {
          self.warn(format!(
            "assembly cycle or depth limit at {path} > {}",
            link.label
          ));
          continue;
        }
        stack.push((
          child_space,
          world * link.transform,
          format!("{path} > {}", link.label),
          ancestors.clone(),
        ));
      }
    }
    if truncated {
      self.warn(format!(
        "more than {MAX_OCCURRENCES} occurrences, truncated"
      ));
    }

    let has_assembly = !nauos.is_empty();
    for (i, reached) in reached.into_iter().enumerate() {
      if reached {
        continue;
      }
      let body = BodyId(i as u32);
      let step_id = self.m.bodies[i].step_id;
      if has_assembly {
        self.warn(format!(
          "body #{step_id} is not placed by the assembly structure, identity occurrence used"
        ));
      }
      self.m.occurrences.push(Occurrence {
        body,
        transform: Mat4::identity(),
        path: format!("#{step_id}"),
      });
    }
  }

  fn item_defined_transformation(&mut self, id: u64) -> R<(Frame, Frame)> {
    let t = self.t;
    let idt = t.item_defined_transformation.get(&id).ok_or_else(|| {
      format!(
        "transformation #{id} is not an ITEM_DEFINED_TRANSFORMATION ({})",
        describe(t, id)
      )
    })?;
    let f1 = self.frame3(&idt.transform_item_1)?;
    let f2 = self.frame3(&idt.transform_item_2)?;
    Ok((f1, f2))
  }

  /// Mapped representation and its transform into the representation holding the item.
  fn mapped_item(&mut self, source: Option<u64>, target: Option<u64>) -> R<(u64, Mat4<f64>)> {
    let t = self.t;
    let source = source.ok_or("inline mapping source")?;
    let target = target.ok_or("inline mapping target")?;
    let map = raw_record(t, source, "REPRESENTATION_MAP").ok_or_else(|| {
      format!(
        "#{source} is not a REPRESENTATION_MAP ({})",
        describe(t, source)
      )
    })?;
    let (Some(origin), Some(mapped)) = (
      map.first().and_then(param_ref),
      map.get(1).and_then(param_ref),
    ) else {
      return Err(format!("REPRESENTATION_MAP #{source} is malformed"));
    };
    let origin = self.frame3_by_id(origin)?;
    let target = self.frame3_by_id(target)?;
    Ok((mapped, target.to_mat4() * origin.to_mat4_inverse()))
  }
}
