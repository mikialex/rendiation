use std::collections::HashMap;

use ruststep::ast::EntityInstance;

use crate::entities::*;

/// Maps STEP entity IDs to their parsed data.
///
/// Each field is a `HashMap<u64, HolderType>` keyed by the entity instance ID
/// from the STEP file. The holder types retain `PlaceHolder::Ref` for
/// cross-references; consumers call `get_owned(id)` to resolve them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
  // primitives
  pub cartesian_point: HashMap<u64, CartesianPointHolder>,
  pub direction: HashMap<u64, DirectionHolder>,
  pub vector: HashMap<u64, VectorHolder>,

  // placements
  pub axis1_placement: HashMap<u64, Axis1PlacementHolder>,
  pub axis2_placement_2d: HashMap<u64, Axis2Placement2dHolder>,
  pub axis2_placement_3d: HashMap<u64, Axis2Placement3dHolder>,

  // curves
  pub line: HashMap<u64, LineHolder>,
  pub polyline: HashMap<u64, PolylineHolder>,
  pub circle: HashMap<u64, CircleHolder>,
  pub ellipse: HashMap<u64, EllipseHolder>,
  pub hyperbola: HashMap<u64, HyperbolaHolder>,
  pub parabola: HashMap<u64, ParabolaHolder>,
  pub b_spline_curve_with_knots: HashMap<u64, BSplineCurveWithKnotsHolder>,
  pub bezier_curve: HashMap<u64, BezierCurveHolder>,
  pub rational_b_spline_curve: HashMap<u64, RationalBSplineCurveHolder>,
  pub trimmed_curve: HashMap<u64, TrimmedCurveHolder>,
  pub composite_curve: HashMap<u64, CompositeCurveHolder>,
  pub composite_curve_segment: HashMap<u64, CompositeCurveSegmentHolder>,
  pub offset_curve_3d: HashMap<u64, OffsetCurve3dHolder>,
  pub pcurve: HashMap<u64, PcurveHolder>,
  pub surface_curve: HashMap<u64, SurfaceCurveHolder>,

  // surfaces
  pub plane: HashMap<u64, PlaneHolder>,
  pub spherical_surface: HashMap<u64, SphericalSurfaceHolder>,
  pub cylindrical_surface: HashMap<u64, CylindricalSurfaceHolder>,
  pub toroidal_surface: HashMap<u64, ToroidalSurfaceHolder>,
  pub conical_surface: HashMap<u64, ConicalSurfaceHolder>,
  pub b_spline_surface_with_knots: HashMap<u64, BSplineSurfaceWithKnotsHolder>,
  pub bezier_surface: HashMap<u64, BezierSurfaceHolder>,
  pub rational_b_spline_surface: HashMap<u64, RationalBSplineSurfaceHolder>,
  pub surface_of_linear_extrusion: HashMap<u64, SurfaceOfLinearExtrusionHolder>,
  pub surface_of_revolution: HashMap<u64, SurfaceOfRevolutionHolder>,
  pub offset_surface: HashMap<u64, OffsetSurfaceHolder>,

  // topology
  pub vertex_point: HashMap<u64, VertexPointHolder>,
  pub edge_curve: HashMap<u64, EdgeCurveHolder>,
  pub oriented_edge: HashMap<u64, OrientedEdgeHolder>,
  pub edge_loop: HashMap<u64, EdgeLoopHolder>,
  pub face_bound: HashMap<u64, FaceBoundHolder>,
  pub face_surface: HashMap<u64, FaceSurfaceHolder>,
  pub oriented_face: HashMap<u64, OrientedFaceHolder>,
  pub shell: HashMap<u64, ShellHolder>,
  pub oriented_shell: HashMap<u64, OrientedShellHolder>,
  pub shell_based_surface_model: HashMap<u64, ShellBasedSurfaceModelHolder>,
  pub manifold_solid_brep: HashMap<u64, ManifoldSolidBrepHolder>,
  pub faceted_brep: HashMap<u64, FacetedBrepHolder>,

  // assembly / navigation
  pub representation: HashMap<u64, RepresentationHolder>,
  pub representation_item: HashMap<u64, RepresentationItemHolder>,
  pub representation_context: HashMap<u64, RepresentationContextHolder>,
  pub mapped_item: HashMap<u64, MappedItemHolder>,
  pub product: HashMap<u64, ProductHolder>,
  pub product_definition_formation: HashMap<u64, ProductDefinitionFormationHolder>,
  pub product_definition_context: HashMap<u64, ProductDefinitionContextHolder>,
  pub product_definition: HashMap<u64, ProductDefinitionHolder>,
  pub product_definition_shape: HashMap<u64, ProductDefinitionShapeHolder>,
  pub shape_definition_representation: HashMap<u64, ShapeDefinitionRepresentationHolder>,
  pub shape_representation: HashMap<u64, ShapeRepresentationHolder>,
  pub context_dependent_shape_representation:
    HashMap<u64, ContextDependentShapeRepresentationHolder>,
  pub shape_representation_relationship: HashMap<u64, ShapeRepresentationRelationshipHolder>,
  pub shape_representation_relationship_with_transformation:
    HashMap<u64, ShapeRepresentationRelationshipWithTransformationHolder>,
  pub next_assembly_usage_occurrence: HashMap<u64, NextAssemblyUsageOccurrenceHolder>,
  pub item_defined_transformation: HashMap<u64, ItemDefinedTransformationHolder>,
  pub geometric_set: HashMap<u64, GeometricSetHolder>,
  pub geometric_curve_set: HashMap<u64, GeometricCurveSetHolder>,
  pub definitional_representation: HashMap<u64, DefinitionalRepresentationHolder>,

  // presentation / visual
  pub styled_item: HashMap<u64, StyledItemHolder>,
  pub presentation_style_assignment: HashMap<u64, PresentationStyleAssignmentHolder>,
  pub surface_style_usage: HashMap<u64, SurfaceStyleUsageHolder>,
  pub surface_side_style: HashMap<u64, SurfaceSideStyleHolder>,
  pub surface_style_fill_area: HashMap<u64, SurfaceStyleFillAreaHolder>,
  pub fill_area_style_colour: HashMap<u64, FillAreaStyleColourHolder>,
  pub colour_rgb: HashMap<u64, ColourRGBHolder>,
  pub surface_style_transparency: HashMap<u64, SurfaceStyleTransparencyHolder>,
  pub surface_style_rendering: HashMap<u64, SurfaceStyleRenderingHolder>,

  // catch-all
  pub unrecognized: HashMap<u64, UnrecognizedEntityHolder>,
}

impl Table {
  /// Parse a STEP exchange structure string into a `Table`.
  pub fn from_step(step_str: &str) -> Option<Self> {
    let exchange = ruststep::parser::parse(step_str).ok()?;
    Some(Self::from_data_section(&exchange.data[0]))
  }

  /// Build a `Table` from the first data section of a STEP file.
  pub fn from_data_section(data_section: &ruststep::ast::DataSection) -> Self {
    Self::from_iter(&data_section.entities)
  }

  /// Dispatch a single entity instance into the appropriate HashMap.
  pub fn push_instance(&mut self, instance: &EntityInstance) -> ruststep::error::Result<()> {
    crate::entities::push_instance(self, instance)
  }
}

impl<'a> FromIterator<&'a EntityInstance> for Table {
  fn from_iter<I: IntoIterator<Item = &'a EntityInstance>>(iter: I) -> Self {
    let mut table = Table::default();
    for instance in iter {
      table
        .push_instance(instance)
        .unwrap_or_else(|e| eprintln!("step-reader: {e}"));
    }
    table
  }
}
