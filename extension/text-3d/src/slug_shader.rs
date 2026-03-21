//! https://github.com/EricLengyel/Slug/blob/main/SlugPixelShader.hlsl
//! https://github.com/diffusionstudio/slug-webgpu/blob/main/src/SlugPixelShader.wgsl

use rendiation_shader_api::*;

#[shader_fn]
fn calc_root_code(y1: Node<f32>, y2: Node<f32>, y3: Node<f32>) -> Node<u32> {
  // Calculate the root eligibility code for a sample-relative quadratic Bézier curve.
  // Extract the signs of the y coordinates of the three control points.

  let i1 = y1.bitcast::<u32>() >> val(31);
  let i2 = y2.bitcast::<u32>() >> val(30);
  let i3 = y3.bitcast::<u32>() >> val(29);

  let shift = (i3 & val(4)) | (((i2 & val(2)) | (i1 & !val(2))) & !val(4));

  // Eligibility is returned in bits 0 and 8.

  (val(0x2E74) >> shift) & val(0x0101)
}

#[shader_fn]
fn solve_horiz_poly(p12: Node<Vec4<f32>>, p3: Node<Vec2<f32>>) -> Node<Vec2<f32>> {
  // Solve for the values of t where the curve crosses y = 0.
  // The quadratic polynomial in t is given by
  //
  //     a t^2 - 2b t + c,
  //
  // where a = p1.y - 2 p2.y + p3.y, b = p1.y - p2.y, and c = p1.y.
  // The discriminant b^2 - ac is clamped to zero, and imaginary
  // roots are treated as a double root at the global minimum
  // where t = b / a.

  let a = vec2_node((
    p12.x() - p12.z() * val(2.0) + p3.x(),
    p12.y() - p12.w() * val(2.0) + p3.y(),
  ));
  let b = vec2_node((p12.x() - p12.z(), p12.y() - p12.w()));
  let ra = val(1.0) / a.y();
  let rb = val(0.5) / b.y();

  let d = (b.y() * b.y() - a.y() * p12.y()).max(val(0.0)).sqrt();

  let t1 = (b.y() - d) * ra;
  let t2 = (b.y() + d) * ra;

  let t1 = t1.make_local_var();
  let t2 = t2.make_local_var();

  // If the polynomial is nearly linear, then solve -2b t + c = 0.
  if_by(a.y().abs().less_than(val(1.0 / 65536.0)), || {
    t1.store(p12.y() * rb);
    t2.store(p12.y() * rb);
  });

  // Return the x coordinates where C(t) = 0.
  let t1 = t1.load();
  let t2 = t2.load();

  (
    (a.x() * t1 - b.x() * val(2.0)) * t1 + p12.x(),
    (a.x() * t2 - b.x() * val(2.0)) * t2 + p12.x(),
  )
    .into()
}

#[shader_fn]
fn solve_vert_poly(p12: Node<Vec4<f32>>, p3: Node<Vec2<f32>>) -> Node<Vec2<f32>> {
  // Solve for the values of t where the curve crosses x = 0.

  let a = vec2_node((
    p12.x() - p12.z() * val(2.0) + p3.x(),
    p12.y() - p12.w() * val(2.0) + p3.y(),
  ));
  let b = vec2_node((p12.x() - p12.z(), p12.y() - p12.w()));

  let ra = val(1.0) / a.x();
  let rb = val(0.5) / b.x();

  let d = (b.x() * b.x() - a.x() * p12.x()).max(val(0.0)).sqrt();
  let t1 = (b.x() - d) * ra;
  let t2 = (b.x() + d) * ra;

  let t1 = t1.make_local_var();
  let t2 = t2.make_local_var();

  // If the polynomial is nearly linear, then solve -2b t + c = 0.

  if_by(a.x().abs().less_than(val(1.0 / 65536.0)), || {
    t1.store(p12.x() * rb);
    t2.store(p12.x() * rb);
  });

  // Return the y coordinates where C(t) = 0.
  let t1 = t1.load();
  let t2 = t2.load();

  (
    (a.y() * t1 - b.y() * val(2.0)) * t1 + p12.y(),
    (a.y() * t2 - b.y() * val(2.0)) * t2 + p12.y(),
  )
    .into()
}

fn calc_band_loc(glyphLoc: Node<Vec2<i32>>, offset: Node<u32>) -> Node<Vec2<i32>> {
  // If the offset causes the x coordinate to exceed the texture width, then wrap to the next line.

  let k_log_band_texture_width: u32 = 12;

  let band_loc = vec2_node((glyphLoc.x() + offset.into_i32(), glyphLoc.y()));
  let y = band_loc.y() + (band_loc.x() >> val(k_log_band_texture_width as i32));
  let x = band_loc.x() & val((1 << k_log_band_texture_width) - 1);

  (x, y).into()
}

// Override constants for optional features.
// Set SLUG_EVENODD = true to enable even-odd fill rule support.
// Set SLUG_WEIGHT = true to enable optical weight boost via square root.

const SLUG_EVENODD: bool = false;
const SLUG_WEIGHT: bool = false;

#[shader_fn]
fn calc_coverage(
  xcov: Node<f32>,
  ycov: Node<f32>,
  xwgt: Node<f32>,
  ywgt: Node<f32>,
  flags: Node<i32>,
) -> Node<f32> {
  // Combine coverages from the horizontal and vertical rays using their weights.
  // Absolute values ensure that either winding direction convention works.

  let cov_abs_min = xcov.abs().min(ycov.abs());
  let cov_ = xcov * xwgt + ycov * ywgt;
  let cov_ = cov_.abs() / (xwgt + ywgt).max(1.0 / 65536.0);

  let coverage = cov_.max(cov_abs_min).make_local_var();

  // If SLUG_EVENODD is defined during compilation, then check E flag in tex.w. (See vertex shader.)

  if SLUG_EVENODD {
    let c = coverage.load();

    if_by((flags & val(0x1000)).equals(val(0)), || {
      // Using nonzero fill rule here.
      coverage.store(coverage.load().saturate());
    })
    .else_by(|| {
      // Using even-odd fill rule here.
      let c_ = (c * val(0.5)).fract() * val(2.);
      coverage.store(val(1.0) - c_.abs());
    });
  } else {
    // Using nonzero fill rule here.

    coverage.store(coverage.load().saturate());
  }

  // If SLUG_WEIGHT is defined during compilation, then take a square root to boost optical weight.

  if SLUG_WEIGHT {
    coverage.store(coverage.load().sqrt());
  }

  coverage.load()
}

fn SlugRender(
  curve_data: BindingNode<ShaderTexture2D>,
  band_data: BindingNode<ShaderTexture<TextureDimension2, u32>>,
  render_coord: Node<Vec2<f32>>,
  bandT_transform: Node<Vec4<f32>>,
  glyph_data: Node<Vec4<i32>>,
) -> Node<f32> {
  // The effective pixel dimensions of the em square are computed
  // independently for x and y directions with texcoord derivatives.

  let ems_per_pixel = render_coord.fwidth();
  let pixels_per_em = val(1.0).splat() / ems_per_pixel;

  let band_max = vec2_node((glyph_data.z(), glyph_data.w() & val(0x00FF)));

  // Determine what bands the current pixel lies in by applying a scale and offset
  // to the render coordinates. The scales are given by bandTransform.xy, and the
  // offsets are given by bandTransform.zw. Band indexes are clamped to [0, bandMax.xy].

  let band_index = (render_coord * bandT_transform.xy() + bandT_transform.zw())
    .into_i32()
    .clamp(val(Vec2::zero()), band_max);
  let glyph_loc = vec2_node((glyph_data.x(), glyph_data.y()));

  let xcov = val(0.0).make_local_var();
  let xwgt = val(0.0).make_local_var();

  // Fetch data for the horizontal band from the index texture. The number
  // of curves intersecting the band is in the x component, and the offset
  // to the list of locations for those curves is in the y component.

  let hband_data = band_data
    .load_texel(
      vec2_node((glyph_loc.x() + band_index.y(), glyph_loc.y())).into_u32(),
      0,
    )
    .xy();
  let hband_loc = calc_band_loc(glyph_loc, hband_data.y());

  // Loop over all curves in the horizontal band.

  hband_data
    .x()
    .into_shader_iter()
    .for_each(|curve_index, lcx| {
      let curve_index = curve_index.into_i32();
      // Fetch the location of the current curve from the index texture.
      let curve_loc = band_data
        .load_texel(
          vec2_node((hband_loc.x() + curve_index, hband_loc.y())).into_u32(),
          0,
        )
        .xy();

      // Fetch the three 2D control points for the current curve from the curve texture.
      // The first texel contains both p1 and p2 in the (x,y) and (z,w) components, respectively,
      // and the the second texel contains p3 in the (x,y) components. Subtracting the render
      // coordinates makes the curve relative to the sample position. The quadratic Bézier curve
      // C(t) is given by
      //
      //     C(t) = (1 - t)^2 p1 + 2t(1 - t) p2 + t^2 p3

      let p12 = curve_data.load_texel(curve_loc, 0) - vec4_node((render_coord, render_coord));
      let p3_coord = vec2_node((curve_loc.x() + val(1), curve_loc.y()));
      let p3 = curve_data.load_texel(p3_coord, 0).xy() - render_coord;

      // If the largest x coordinate among all three control points falls
      // left of the current pixel, then there are no more curves in the
      // horizontal band that can influence the result, so exit the loop.
      // (The curves are sorted in descending order by max x coordinate.)

      let cond = (p12.x().max(p12.z()).max(p3.x()) * pixels_per_em.x()).less_than(val(-0.5));
      if_by(cond, || lcx.do_break());

      let code = calc_root_code_fn(p12.y(), p12.w(), p3.y());
      if_by(code.not_equals(0), || {
        // At least one root makes a contribution. Calculate them and scale so
        // that the current pixel corresponds to the range [0,1].

        let r = solve_horiz_poly_fn(p12, p3) * pixels_per_em.x();

        // Bits in code tell which roots make a contribution.

        if_by((code & val(1)).not_equals(0), || {
          xcov.store(xcov.load() + (r.x() + val(0.5)).saturate());
          let xwgt_next = (val(1.) - r.x().abs() * val(2.)).saturate();
          xwgt.store(xwgt.load().max(xwgt_next));
        });

        if_by(code.greater_than(1), || {
          xcov.store(xcov.load() - (r.y() + val(0.5)).saturate());
          let xwgt_next = (val(1.) - r.y().abs() * val(2.)).saturate();
          xwgt.store(xwgt.load().max(xwgt_next));
        });
      });
    });

  let ycov = val(0.0).make_local_var();
  let ywgt = val(0.0).make_local_var();

  // Fetch data for the vertical band from the index texture. This follows
  // the data for all horizontal bands, so we have to add bandMax.y + 1.

  let coord = vec2_node((
    glyph_loc.x() + band_index.y() + val(1) + band_index.x(),
    glyph_loc.y(),
  ));

  let vband_data = band_data.load_texel(coord.into_u32(), 0).xy();
  let vband_loc = calc_band_loc(glyph_loc, vband_data.y());

  // Loop over all curves in the vertical band.
  vband_data
    .x()
    .into_shader_iter()
    .for_each(|curve_index, lcx| {
      let curve_index = curve_index.into_i32();
      let curve_loc = band_data
        .load_texel(
          vec2_node((vband_loc.x() + curve_index, vband_loc.y())).into_u32(),
          0,
        )
        .xy();

      let p12 = curve_data.load_texel(curve_loc, 0) - vec4_node((render_coord, render_coord));
      let p3_coord = vec2_node((curve_loc.x() + val(1), curve_loc.y()));
      let p3 = curve_data.load_texel(p3_coord, 0).xy() - render_coord;

      // If the largest y coordinate among all three control points falls
      // below the current pixel, then there are no more curves in the
      // vertical band that can influence the result, so exit the loop.
      // (The curves are sorted in descending order by max y coordinate.)

      let cond = (p12.y().max(p12.w()).max(p3.y()) * pixels_per_em.y()).less_than(val(-0.5));
      if_by(cond, || lcx.do_break());

      let code = calc_root_code_fn(p12.x(), p12.z(), p3.x());

      if_by(code.not_equals(0), || {
        let r = solve_vert_poly_fn(p12, p3) * pixels_per_em.y();

        if_by((code & val(1)).not_equals(0), || {
          ycov.store(ycov.load() - (r.x() + val(0.5)).saturate());
          let ywgt_next = (val(1.) - r.x().abs() * val(2.)).saturate();
          ywgt.store(ywgt.load().max(ywgt_next));
        });

        if_by(code.greater_than(1), || {
          ycov.store(ycov.load() + (r.y() + val(0.5)).saturate());
          let ywgt_next = (val(1.) - r.y().abs() * val(2.)).saturate();
          ywgt.store(ywgt.load().max(ywgt_next));
        });
      });
    });

  return calc_coverage_fn(
    xcov.load(),
    ycov.load(),
    xwgt.load(),
    ywgt.load(),
    glyph_data.w(),
  );
}
