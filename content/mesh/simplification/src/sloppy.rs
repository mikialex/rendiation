use std::collections::HashSet;

use crate::*;

pub fn simplify_sloppy<V: Positioned<Position = Vec3<f32>>>(
  destination: &mut [u32],
  indices: &[u32],
  vertices: &[V],
  vertex_lock: Option<&[bool]>,
  target_index_count: u32,
  target_error: f32,
  use_absolute_error: bool,
) -> SimplificationResult {
  assert!(indices.len() % 3 == 0);
  assert!(target_index_count <= indices.len() as u32);

  // we expect to get ~2 triangles/vertex in the output
  let target_cell_count = target_index_count / 6;

  let (vertex_scale, vertex_positions) = rescale_positions(vertices);
  let error_scale = if use_absolute_error { vertex_scale } else { 1. };
  let target_error = target_error / error_scale;

  // find the optimal grid size using guided binary search

  let mut vertex_ids = vec![0_u32; vertices.len()];

  // invariant: # of triangles in min_grid <= target_count
  let mut min_grid = (1. / target_error.max(1e-3)) as u32;
  let mut max_grid = 1025;

  let mut min_triangles = 0;
  let mut max_triangles = indices.len() / 3;

  // when we're error-limited, we compute the triangle count for the min size;
  // this accelerates convergence and provides the correct answer when we can't use a larger grid

  // todo, compare to meshopt, we remove the lock check
  if min_grid > 1 {
    compute_vertex_ids(&mut vertex_ids, &vertex_positions, vertex_lock, min_grid);
    min_triangles = count_triangles(&vertex_ids, indices);
  }

  // instead of starting in the middle, let's guess as to what the answer might be!
  // triangle count usually grows as a square of grid size...
  let mut next_grid_size = ((target_cell_count as f32).sqrt() + 0.5) as u32;

  let interpolation_passes = 5;
  for pass in 0..(10 + interpolation_passes) {
    if min_triangles >= target_index_count / 3 || max_grid - min_grid <= 1 {
      break;
    }

    // we clamp the prediction of the grid size to make sure that the search converges
    let grid_size = next_grid_size;
    let grid_size = if grid_size <= min_grid {
      min_grid + 1
    } else if grid_size >= max_grid {
      max_grid - 1
    } else {
      grid_size
    };

    compute_vertex_ids(&mut vertex_ids, &vertex_positions, vertex_lock, grid_size);
    let triangles = count_triangles(&vertex_ids, indices);

    let tip = interpolate(
      target_index_count as f32 / 3.,
      min_grid as f32,
      min_triangles as f32,
      grid_size as f32,
      triangles as f32,
      max_grid as f32,
      max_triangles as f32,
    );

    if triangles <= target_index_count / 3 {
      min_grid = grid_size;
      min_triangles = triangles;
    } else {
      max_grid = grid_size;
      max_triangles = triangles as usize;
    }

    // we start by using interpolation search - it usually converges faster
    // however, interpolation search has a worst case of O(N) so we switch to binary search after a few iterations which converges in O(logN)
    next_grid_size = if pass < interpolation_passes {
      (tip + 0.5) as u32
    } else {
      (min_grid + max_grid) / 2
    }
  }

  if min_triangles == 0 {
    return SimplificationResult {
      result_error: error_scale,
      result_count: 0,
    };
  }

  // build vertex->cell association by mapping all vertices with the same quantized position to the same cell
  let mut vertex_cells = vec![0_u32; vertices.len()];

  compute_vertex_ids(&mut vertex_ids, &vertex_positions, vertex_lock, min_grid);
  let cell_count = fill_vertex_cells(&mut vertex_cells, &vertex_ids);

  // build a quadric for each target cell
  let mut cell_quadrics = vec![Quadric::default(); cell_count as usize];

  fill_cell_quadrics(
    &mut cell_quadrics,
    indices,
    &vertex_positions,
    &vertex_cells,
  );

  // // for each target cell, find the vertex with the minimal error
  let mut cell_remap = vec![0_u32; cell_count as usize];
  let mut cell_errors = vec![0_f32; cell_count as usize];

  fill_cell_remap(
    &mut cell_remap,
    &mut cell_errors,
    &vertex_cells,
    &cell_quadrics,
    &vertex_positions,
  );

  // compute error
  let mut result_error = 0.;
  for err in &cell_errors {
    result_error = result_error.max(*err);
  }

  // vertex collapses often result in duplicate triangles; we need filter them out
  let write = filter_triangles(destination, &indices, &vertex_cells, &cell_remap);

  SimplificationResult {
    result_error: result_error.sqrt() * error_scale,
    result_count: write,
  }
}

fn compute_vertex_ids(
  vertex_ids: &mut [u32],
  vertex_positions: &[Vec3<f32>],
  vertex_lock: Option<&[bool]>,
  grid_size: u32,
) {
  assert!((1..=1024).contains(&grid_size));
  let cell_scale = grid_size as f32 - 1.;

  for i in 0..vertex_positions.len() {
    let v = vertex_positions[i];
    let v = v.map(|v| (v * cell_scale + 0.5) as i32);
    if let Some(vertex_lock) = vertex_lock {
      if vertex_lock[i] {
        vertex_ids[i] = (1 << 30) | i as u32;
      } else {
        vertex_ids[i] = ((v.x << 20) | (v.y << 10) | v.z) as u32;
      }
    }
  }
}

pub fn count_triangles(vertex_ids: &[u32], indices: &[u32]) -> u32 {
  let mut result = 0;

  for [a, b, c] in indices.array_chunks::<3>() {
    let id0 = vertex_ids[*a as usize];
    let id1 = vertex_ids[*b as usize];
    let id2 = vertex_ids[*c as usize];

    result += (id0 != id1) as u32 & (id0 != id2) as u32 & (id1 != id2) as u32;
  }
  result
}

fn interpolate(y: f32, x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
  // three point interpolation from "revenge of interpolation search" paper
  let num = (y1 - y) * (x1 - x2) * (x1 - x0) * (y2 - y0);
  let den = (y2 - y) * (x1 - x2) * (y0 - y1) + (y0 - y) * (x1 - x0) * (y1 - y2);
  x1 + num / den
}

pub fn fill_cell_quadrics(
  cell_quadrics: &mut [Quadric],
  indices: &[u32],
  vertex_positions: &[Vec3<f32>],
  vertex_cells: &[u32],
) {
  for [i0, i1, i2] in indices.array_chunks::<3>() {
    let c0 = vertex_cells[*i0 as usize];
    let c1 = vertex_cells[*i1 as usize];
    let c2 = vertex_cells[*i2 as usize];

    let p0 = vertex_positions[*i0 as usize];
    let p1 = vertex_positions[*i1 as usize];
    let p2 = vertex_positions[*i2 as usize];

    let single_cell = (c0 == c1) && (c0 == c2);

    let weight = if single_cell { 3. } else { 1. };
    let q = Quadric::from_triangle(p0, p1, p2, weight);

    if single_cell {
      cell_quadrics[c0 as usize] += q;
    } else {
      cell_quadrics[c0 as usize] += q;
      cell_quadrics[c1 as usize] += q;
      cell_quadrics[c2 as usize] += q;
    }
  }
}

pub fn fill_cell_remap(
  cell_remap: &mut [u32],
  cell_errors: &mut [f32],
  vertex_cells: &[u32],
  cell_quadrics: &[Quadric],
  vertex_positions: &[Vec3<f32>],
) {
  cell_remap.fill(u32::MAX);
  for (i, cell) in vertex_cells.iter().enumerate() {
    let cell_ = *cell as usize;
    let error = cell_quadrics[cell_].error(&vertex_positions[i]);

    if cell_remap[cell_] == u32::MAX || cell_errors[cell_] > error {
      cell_remap[cell_] = i as u32;
      cell_errors[cell_] = error;
    }
  }
}

pub fn fill_vertex_cells(vertex_cells: &mut [u32], vertex_ids: &[u32]) -> u32 {
  let mut result = 0;
  let mut map = HashMap::<u32, u32>::default();
  for (i, vertex_id) in vertex_ids.iter().enumerate() {
    match map.entry(*vertex_id) {
      Entry::Vacant(entry) => {
        entry.insert(i as u32);
        vertex_cells[i] = result;
        result += 1;
      }
      Entry::Occupied(entry) => {
        vertex_cells[i] = vertex_cells[*entry.get() as usize];
      }
    }
  }
  result
}

fn filter_triangles(
  destination: &mut [u32],
  indices: &&[u32],
  vertex_cells: &[u32],
  cell_remap: &[u32],
) -> usize {
  let mut filter = HashSet::<(u32, u32, u32)>::new();
  let mut result = 0;
  for [a, b, c] in indices.array_chunks::<3>() {
    let c0 = vertex_cells[*a as usize];
    let c1 = vertex_cells[*b as usize];
    let c2 = vertex_cells[*c as usize];

    if c0 != c1 && c0 != c2 && c1 != c2 {
      let mut a = cell_remap[c0 as usize];
      let mut b = cell_remap[c1 as usize];
      let mut c = cell_remap[c2 as usize];

      if b < a && b < c {
        let t = a;
        a = b;
        b = c;
        c = t;
      } else if c < a && c < b {
        let t = c;
        c = b;
        b = a;
        a = t;
      }

      if filter.insert((a, b, c)) {
        result += 1;
        destination[result * 3] = a;
        destination[result * 3 + 1] = b;
        destination[result * 3 + 2] = c;
      }
    }
  }
  result * 3
}
