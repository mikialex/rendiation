use std::env;
use std::fs;
use std::path::Path;

use rendiation_step_reader::step_utils::{normalize_step, visit_stp_files};
use rendiation_step_reader::table::Table;

/// Parse STEP files and print entity statistics.
///
/// # Usage
///
/// ```sh
/// # Single file: parse and print per-category entity counts
/// cargo run --example parse_step -- path/to/model.stp
///
/// # Directory: recursively scan all .stp/.step files, batch test
/// cargo run --example parse_step -- path/to/dir/
/// ```
///
/// **Single-file mode** prints a breakdown of every entity category
/// (e.g. `cartesian_point: 1234`, `b_spline_surface_with_knots: 5`),
/// plus an `unrecognized` catch-all count.
///
/// **Directory mode** recursively walks all `.stp`/`.step` files,
/// printing `OK`/`FAIL`/`SKIP` per file with entity count and parse
/// time, followed by a pass/fail summary.
fn main() {
  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    eprintln!("usage: {} <file.stp | directory>", args[0]);
    std::process::exit(1);
  }

  let path = Path::new(&args[1]);
  if !path.exists() {
    eprintln!("path does not exist: {}", path.display());
    std::process::exit(1);
  }

  if path.is_dir() {
    let mut total = 0usize;
    let mut ok = 0usize;
    visit_stp_files(path, &mut |file_path| {
      total += 1;
      match parse_file(file_path) {
        ParseResult::Success {
          entity_count,
          duration,
        } => {
          ok += 1;
          println!(
            "OK  {}  ({} entities, {:.0}ms)",
            file_path.display(),
            entity_count,
            duration.as_secs_f64() * 1000.0
          );
        }
        ParseResult::NoDataSection => {
          println!("SKIP {}  (no data section)", file_path.display());
        }
        ParseResult::ParseError(e) => {
          println!("FAIL {}  ({})", file_path.display(), e);
        }
      }
    });
    println!("───");
    println!(
      "scanned {total} file(s), {ok} passed, {} failed",
      total - ok
    );
  } else {
    match parse_file(path) {
      ParseResult::Success {
        entity_count,
        duration,
      } => {
        print_stats(path);
        println!(
          "parsed {} entities in {:.0}ms",
          entity_count,
          duration.as_secs_f64() * 1000.0
        );
      }
      ParseResult::NoDataSection => {
        eprintln!("file has no data section: {}", path.display());
        std::process::exit(1);
      }
      ParseResult::ParseError(e) => {
        eprintln!("parse error in {}: {}", path.display(), e);
        std::process::exit(1);
      }
    }
  }
}

enum ParseResult {
  Success {
    entity_count: usize,
    duration: std::time::Duration,
  },
  NoDataSection,
  ParseError(String),
}

fn parse_file(path: &Path) -> ParseResult {
  let step_str = match fs::read_to_string(path) {
    Ok(s) => s,
    Err(e) => return ParseResult::ParseError(format!("cannot read file: {e}")),
  };

  let step_str = normalize_step(&step_str);

  let start = std::time::Instant::now();

  let exchange = match ruststep::parser::parse(&step_str) {
    Ok(e) => e,
    Err(e) => return ParseResult::ParseError(format!("{e}")),
  };

  if exchange.data.is_empty() {
    return ParseResult::NoDataSection;
  }

  let table = Table::from_data_section(&exchange.data[0]);
  let entity_count = count_all(&table);
  let duration = start.elapsed();

  ParseResult::Success {
    entity_count,
    duration,
  }
}

fn print_stats(path: &Path) {
  let step_str = match fs::read_to_string(path) {
    Ok(s) => normalize_step(&s),
    Err(e) => {
      eprintln!("cannot read file: {e}");
      return;
    }
  };

  let exchange = match ruststep::parser::parse(&step_str) {
    Ok(e) => e,
    Err(e) => {
      eprintln!("parse error: {e}");
      return;
    }
  };

  if exchange.data.is_empty() {
    return;
  }

  let table = Table::from_data_section(&exchange.data[0]);

  macro_rules! stat {
    ($field:ident, $label:expr) => {
      let n = table.$field.len();
      if n > 0 {
        println!("  {:>6}  {}", n, $label);
      }
    };
  }

  println!("{}", path.display());
  println!("  entities by category:");

  // primitives
  stat!(cartesian_point, "cartesian_point");
  stat!(direction, "direction");
  stat!(vector, "vector");

  // placements
  stat!(axis1_placement, "axis1_placement");
  stat!(axis2_placement_2d, "axis2_placement_2d");
  stat!(axis2_placement_3d, "axis2_placement_3d");

  // curves
  stat!(line, "line");
  stat!(polyline, "polyline");
  stat!(circle, "circle");
  stat!(ellipse, "ellipse");
  stat!(hyperbola, "hyperbola");
  stat!(parabola, "parabola");
  stat!(b_spline_curve_with_knots, "b_spline_curve_with_knots");
  stat!(bezier_curve, "bezier_curve");
  stat!(rational_b_spline_curve, "rational_b_spline_curve");
  stat!(trimmed_curve, "trimmed_curve");
  stat!(composite_curve, "composite_curve");
  stat!(offset_curve_3d, "offset_curve_3d");
  stat!(pcurve, "pcurve");
  stat!(surface_curve, "surface_curve");

  // surfaces
  stat!(plane, "plane");
  stat!(spherical_surface, "spherical_surface");
  stat!(cylindrical_surface, "cylindrical_surface");
  stat!(toroidal_surface, "toroidal_surface");
  stat!(conical_surface, "conical_surface");
  stat!(b_spline_surface_with_knots, "b_spline_surface_with_knots");
  stat!(bezier_surface, "bezier_surface");
  stat!(rational_b_spline_surface, "rational_b_spline_surface");
  stat!(surface_of_linear_extrusion, "surface_of_linear_extrusion");
  stat!(surface_of_revolution, "surface_of_revolution");
  stat!(offset_surface, "offset_surface");

  // topology
  stat!(vertex_point, "vertex_point");
  stat!(edge_curve, "edge_curve");
  stat!(oriented_edge, "oriented_edge");
  stat!(edge_loop, "edge_loop");
  stat!(face_bound, "face_bound");
  stat!(face_surface, "face_surface");
  stat!(oriented_face, "oriented_face");
  stat!(shell, "shell");
  stat!(oriented_shell, "oriented_shell");
  stat!(shell_based_surface_model, "shell_based_surface_model");
  stat!(manifold_solid_brep, "manifold_solid_brep");
  stat!(faceted_brep, "faceted_brep");

  // assembly
  stat!(representation, "representation");
  stat!(representation_item, "representation_item");
  stat!(representation_context, "representation_context");
  stat!(mapped_item, "mapped_item");
  stat!(product, "product");
  stat!(product_definition_formation, "product_definition_formation");
  stat!(product_definition, "product_definition");
  stat!(product_definition_shape, "product_definition_shape");
  stat!(
    shape_definition_representation,
    "shape_definition_representation"
  );
  stat!(shape_representation, "shape_representation");
  stat!(
    next_assembly_usage_occurrence,
    "next_assembly_usage_occurrence"
  );
  stat!(item_defined_transformation, "item_defined_transformation");
  stat!(geometric_set, "geometric_set");
  stat!(geometric_curve_set, "geometric_curve_set");

  // visual
  stat!(styled_item, "styled_item");
  stat!(
    presentation_style_assignment,
    "presentation_style_assignment"
  );
  stat!(surface_style_usage, "surface_style_usage");
  stat!(surface_side_style, "surface_side_style");
  stat!(surface_style_fill_area, "surface_style_fill_area");
  stat!(fill_area_style_colour, "fill_area_style_colour");
  stat!(colour_rgb, "colour_rgb");
  stat!(surface_style_transparency, "surface_style_transparency");
  stat!(surface_style_rendering, "surface_style_rendering");

  // catch-all
  stat!(unrecognized, "unrecognized (unhandled entities)");

  let total = count_all(&table);
  println!("  ─────");
  println!("  {:>6}  total", total);
}

fn count_all(table: &Table) -> usize {
  table.cartesian_point.len()
    + table.direction.len()
    + table.vector.len()
    + table.axis1_placement.len()
    + table.axis2_placement_2d.len()
    + table.axis2_placement_3d.len()
    + table.line.len()
    + table.polyline.len()
    + table.circle.len()
    + table.ellipse.len()
    + table.hyperbola.len()
    + table.parabola.len()
    + table.b_spline_curve_with_knots.len()
    + table.bezier_curve.len()
    + table.rational_b_spline_curve.len()
    + table.trimmed_curve.len()
    + table.composite_curve.len()
    + table.offset_curve_3d.len()
    + table.pcurve.len()
    + table.surface_curve.len()
    + table.plane.len()
    + table.spherical_surface.len()
    + table.cylindrical_surface.len()
    + table.toroidal_surface.len()
    + table.conical_surface.len()
    + table.b_spline_surface_with_knots.len()
    + table.bezier_surface.len()
    + table.rational_b_spline_surface.len()
    + table.surface_of_linear_extrusion.len()
    + table.surface_of_revolution.len()
    + table.offset_surface.len()
    + table.vertex_point.len()
    + table.edge_curve.len()
    + table.oriented_edge.len()
    + table.edge_loop.len()
    + table.face_bound.len()
    + table.face_surface.len()
    + table.oriented_face.len()
    + table.shell.len()
    + table.oriented_shell.len()
    + table.shell_based_surface_model.len()
    + table.manifold_solid_brep.len()
    + table.faceted_brep.len()
    + table.representation.len()
    + table.representation_item.len()
    + table.representation_context.len()
    + table.mapped_item.len()
    + table.product.len()
    + table.product_definition_formation.len()
    + table.product_definition_context.len()
    + table.product_definition.len()
    + table.product_definition_shape.len()
    + table.shape_definition_representation.len()
    + table.shape_representation.len()
    + table.context_dependent_shape_representation.len()
    + table.shape_representation_relationship.len()
    + table
      .shape_representation_relationship_with_transformation
      .len()
    + table.next_assembly_usage_occurrence.len()
    + table.item_defined_transformation.len()
    + table.geometric_set.len()
    + table.geometric_curve_set.len()
    + table.definitional_representation.len()
    + table.styled_item.len()
    + table.presentation_style_assignment.len()
    + table.surface_style_usage.len()
    + table.surface_side_style.len()
    + table.surface_style_fill_area.len()
    + table.fill_area_style_colour.len()
    + table.colour_rgb.len()
    + table.surface_style_transparency.len()
    + table.surface_style_rendering.len()
    + table.unrecognized.len()
}
