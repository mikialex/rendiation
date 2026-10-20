//! Topology resolution: bodies, shells, faces, bounds, edges and vertices.

use super::builder::*;
use super::lookup::*;
use super::*;

impl Builder<'_> {
  /// Every solid / surface model of the file becomes a body, in STEP id order.
  pub fn resolve_bodies(&mut self) {
    let t = self.t;
    for id in sorted_ids(&t.manifold_solid_brep) {
      let brep = &t.manifold_solid_brep[&id];
      let mut shells = Vec::new();
      match ref_or_err(&brep.outer, "shell").and_then(|s| self.shell(s, true)) {
        Ok(faces) => shells.push(faces),
        Err(e) => self.warn(format!("solid #{id}: outer shell skipped: {e}")),
      }
      for void in &brep.voids {
        match ref_or_err(void, "void shell").and_then(|s| self.oriented_shell(s)) {
          Ok(faces) => shells.push(faces),
          Err(e) => self.warn(format!("solid #{id}: void shell skipped: {e}")),
        }
      }
      self.push_body(id, shells);
    }
    for id in sorted_ids(&t.shell_based_surface_model) {
      let model = &t.shell_based_surface_model[&id];
      let mut shells = Vec::new();
      for shell in &model.sbms_boundary {
        match ref_or_err(shell, "shell").and_then(|s| self.oriented_shell(s)) {
          Ok(faces) => shells.push(faces),
          Err(e) => self.warn(format!("surface model #{id}: shell skipped: {e}")),
        }
      }
      self.push_body(id, shells);
    }
    for id in sorted_ids(&t.faceted_brep) {
      let brep = &t.faceted_brep[&id];
      let shells = match ref_or_err(&brep.outer, "shell").and_then(|s| self.shell(s, true)) {
        Ok(faces) => vec![faces],
        Err(e) => {
          self.warn(format!("faceted brep #{id}: shell skipped: {e}"));
          Vec::new()
        }
      };
      self.push_body(id, shells);
    }
  }

  fn push_body(&mut self, step_id: u64, shells: Vec<Vec<(FaceId, bool)>>) {
    if shells.iter().all(|s| s.is_empty()) {
      self.warn(format!("body #{step_id} has no resolvable face"));
    }
    self.m.bodies.push(Body { step_id, shells });
    self
      .bodies
      .insert(step_id, BodyId(self.m.bodies.len() as u32 - 1));
  }

  /// A shell referenced where an oriented shell is allowed (`ORIENTED_CLOSED_SHELL`,
  /// `ORIENTED_OPEN_SHELL`, or a plain shell).
  fn oriented_shell(&mut self, id: u64) -> R<Vec<(FaceId, bool)>> {
    if let Some(os) = self.t.oriented_shell.get(&id) {
      let inner = ref_or_err(&os.shell_element, "shell")?;
      return self.shell(inner, os.orientation);
    }
    self.shell(id, true)
  }

  /// Faces of a shell with their effective orientation: `ORIENTED_FACE.orientation`, combined
  /// with the orientation of an enclosing oriented shell (which reverses every face, per
  /// `conditional_reverse` in ISO 10303-42). Unresolvable faces are skipped with a warning.
  fn shell(&mut self, id: u64, orientation: bool) -> R<Vec<(FaceId, bool)>> {
    let t = self.t;
    let shell = t
      .shell
      .get(&id)
      .ok_or_else(|| format!("#{id} is not a shell ({})", describe(t, id)))?;
    let mut faces = Vec::with_capacity(shell.shell_element.len());
    for element in &shell.shell_element {
      let Some(element) = ref_id(element) else {
        self.warn(format!("shell #{id}: inline face skipped"));
        continue;
      };
      let (face_id, face_orientation) = if let Some(of) = t.oriented_face.get(&element) {
        match ref_id(&of.face_element) {
          Some(f) => (f, of.orientation),
          None => {
            self.warn(format!(
              "shell #{id}: oriented face #{element} has an inline face"
            ));
            continue;
          }
        }
      } else {
        (element, true)
      };
      if let Some(face) = self.face(face_id) {
        faces.push((face, face_orientation == orientation));
      }
    }
    Ok(faces)
  }

  /// Dense id of a face, `None` (with a warning, once) if it can not be resolved.
  fn face(&mut self, id: u64) -> Option<FaceId> {
    if let Some(f) = self.faces.get(&id) {
      return *f;
    }
    let r = match self.build_face(id) {
      Ok(face) => {
        self.m.faces.push(face);
        Some(FaceId(self.m.faces.len() as u32 - 1))
      }
      Err(e) => {
        self.warn(format!("face #{id} skipped: {e}"));
        None
      }
    };
    self.faces.insert(id, r);
    r
  }

  fn build_face(&mut self, id: u64) -> R<Face> {
    let t = self.t;
    let face = t
      .face_surface
      .get(&id)
      .ok_or_else(|| format!("#{id} is not a face ({})", describe(t, id)))?;
    let surface = self.surface(ref_or_err(&face.face_geometry, "surface")?)?;
    let mut bounds = Vec::with_capacity(face.bounds.len());
    for bound in &face.bounds {
      bounds.push(self.face_bound(ref_or_err(bound, "face bound")?)?);
    }
    Ok(Face {
      step_id: id,
      surface,
      same_sense: face.same_sense,
      bounds,
    })
  }

  fn face_bound(&mut self, id: u64) -> R<FaceBound> {
    let t = self.t;
    let bound = t
      .face_bound
      .get(&id)
      .ok_or_else(|| format!("#{id} is not a face bound ({})", describe(t, id)))?;
    let is_outer = t.subtype_name.get(&id).map(|n| n.as_str()) == Some("FACE_OUTER_BOUND");
    let loop_id = ref_or_err(&bound.bound, "loop")?;
    let kind = if let Some(el) = t.edge_loop.get(&loop_id) {
      if el.edge_list.is_empty() {
        return Err(format!("edge loop #{loop_id} is empty"));
      }
      let mut edges = Vec::with_capacity(el.edge_list.len());
      for oe in &el.edge_list {
        let oe_id = ref_or_err(oe, "oriented edge")?;
        let oe = t.oriented_edge.get(&oe_id).ok_or_else(|| {
          format!(
            "loop #{loop_id}: #{oe_id} is not an oriented edge ({})",
            describe(t, oe_id)
          )
        })?;
        let edge = self.edge(ref_or_err(&oe.edge_element, "edge")?)?;
        edges.push(OrientedEdgeUse {
          edge,
          orientation: oe.orientation,
        });
      }
      BoundKind::Edges(edges)
    } else if let Some(vl) = t.vertex_loop.get(&loop_id) {
      BoundKind::Vertex(self.vertex(ref_or_err(&vl.loop_vertex, "vertex")?)?)
    } else {
      return Err(format!(
        "bound #{id}: loop #{loop_id} is not supported ({})",
        describe(t, loop_id)
      ));
    };
    Ok(FaceBound {
      step_id: id,
      is_outer,
      orientation: bound.orientation,
      kind,
    })
  }

  /// Dense id of an edge. A failure is warned once here; every face using the edge reports
  /// its own skip.
  fn edge(&mut self, id: u64) -> R<EdgeId> {
    if let Some(r) = self.edges.get(&id) {
      return r.clone().map_err(|_| format!("uses skipped edge #{id}"));
    }
    let r = self.build_edge(id).map(|edge| {
      self.m.edges.push(edge);
      EdgeId(self.m.edges.len() as u32 - 1)
    });
    if let Err(e) = &r {
      self.warn(format!("edge #{id} skipped: {e}"));
    }
    self.edges.insert(id, r.clone());
    r.map_err(|_| format!("uses skipped edge #{id}"))
  }

  fn build_edge(&mut self, id: u64) -> R<Edge> {
    let t = self.t;
    let ec = t
      .edge_curve
      .get(&id)
      .ok_or_else(|| format!("#{id} is not an edge curve ({})", describe(t, id)))?;
    let start = self.vertex(ref_or_err(&ec.edge_start, "vertex")?)?;
    let end = self.vertex(ref_or_err(&ec.edge_end, "vertex")?)?;
    let geometry = ref_or_err(&ec.edge_geometry, "edge geometry")?;

    let mut pcurves = Vec::new();
    let mut is_seam = false;
    let curve = if let Some(sc) = t.surface_curve.get(&geometry) {
      is_seam = t
        .subtype_name
        .get(&geometry)
        .is_some_and(|n| n.contains("SEAM_CURVE"));
      for assoc in &sc.associated_geometry {
        let Some(assoc) = ref_id(assoc) else {
          continue;
        };
        // Associated geometry may also be a plain surface, which carries no pcurve.
        if t.pcurve.contains_key(&assoc) {
          match self.pcurve(assoc) {
            Ok(p) => pcurves.push(p),
            Err(e) => self.warn(format!("edge #{id}: pcurve #{assoc} skipped: {e}")),
          }
        }
      }
      self.curve3(ref_or_err(&sc.curve_3d, "curve_3d")?)?
    } else if t.pcurve.contains_key(&geometry) {
      // A pcurve as the only edge geometry: no 3D curve, keep the pcurve.
      match self.pcurve(geometry) {
        Ok(p) => pcurves.push(p),
        Err(e) => self.warn(format!("edge #{id}: pcurve #{geometry} skipped: {e}")),
      }
      self.curve3(geometry)?
    } else {
      self.curve3(geometry)?
    };
    Ok(Edge {
      step_id: id,
      start,
      end,
      curve,
      same_sense: ec.same_sense,
      pcurves,
      is_seam,
    })
  }

  /// `PCURVE`: basis surface plus the 2D curve item of its `DEFINITIONAL_REPRESENTATION`.
  fn pcurve(&mut self, id: u64) -> R<PcurveRef> {
    let t = self.t;
    let p = t
      .pcurve
      .get(&id)
      .ok_or_else(|| format!("#{id} is not a pcurve ({})", describe(t, id)))?;
    let surface = self.surface(ref_or_err(&p.basis_surface, "basis surface")?)?;
    let rep_id = ref_or_err(&p.reference_to_curve, "definitional representation")?;
    let rep = t.definitional_representation.get(&rep_id).ok_or_else(|| {
      format!(
        "#{rep_id} is not a definitional representation ({})",
        describe(t, rep_id)
      )
    })?;
    let item = match rep.representation_item.as_slice() {
      [item] => ref_or_err(item, "2D curve")?,
      items => {
        return Err(format!(
          "definitional representation #{rep_id} has {} items, 1 expected",
          items.len()
        ));
      }
    };
    let scale = self.pcurve_scale(surface);
    Ok(PcurveRef {
      surface,
      curve: self.curve2(item, scale)?,
      step_id: id,
    })
  }

  fn vertex(&mut self, id: u64) -> R<VertexId> {
    if let Some(r) = self.vertices.get(&id) {
      return r.clone();
    }
    let t = self.t;
    let r = t
      .vertex_point
      .get(&id)
      .ok_or_else(|| format!("#{id} is not a vertex point ({})", describe(t, id)))
      .and_then(|v| self.point3(&v.vertex_geometry))
      .map(|position| {
        self.m.vertices.push(Vertex {
          step_id: id,
          position,
        });
        VertexId(self.m.vertices.len() as u32 - 1)
      })
      .map_err(|e| format!("vertex #{id}: {e}"));
    self.vertices.insert(id, r.clone());
    r
  }
}
