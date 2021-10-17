use std::collections::HashMap;

use linked_hash_map::LinkedHashMap;
use rendiation_texture::TextureRange;

use crate::*;

pub struct ShelfPacker {
  config: PackerConfig,
  rows: LinkedHashMap<usize, Row>,
  /// Mapping of row gaps bottom -> top
  space_start_for_end: HashMap<usize, usize>,
  /// Mapping of row gaps top -> bottom
  space_end_for_start: HashMap<usize, usize>,

  packed: HashMap<PackId, (usize, usize)>,
}

impl ShelfPacker {
  pub fn new(config: PackerConfig) -> Self {
    ShelfPacker {
      config,
      rows: Default::default(),
      space_start_for_end: Default::default(),
      space_end_for_start: Default::default(),
      packed: Default::default(),
    }
  }
}

/// Row of pixel data
struct Row {
  /// Row pixel height
  height: usize,
  /// Pixel width current in use by glyphs
  width: usize,

  items: Vec<TextureRange>,
}

impl BaseTexturePacker for ShelfPacker {
  fn config(&mut self, config: PackerConfig) {
    self.config = config;
    self.reset();
  }

  fn reset(&mut self) {
    *self = Self::new(self.config)
  }
}

/// https://github.com/alexheretic/glyph-brush/blob/master/draw-cache/src/lib.rs
impl RePackablePacker for ShelfPacker {
  fn pack_with_id(
    &mut self,
    input: rendiation_texture::Size,
  ) -> Result<PackResultWithId, PackError> {
    let width = usize::from(input.width);
    let height = usize::from(input.height);

    // find row to put the glyph in, most used rows first
    let mut row_top = self
      .rows
      .iter()
      .find(|(_, row)| row.width >= width && row.height >= height)
      .map(|row| *row.0);

    if let Some(row_top) = row_top {
      // // calculate the target rect
      // let row = self.rows.get_refresh(&row_top).unwrap();

      // let tex_coords = Rectangle {
      //   min: [row.width, row_top],
      //   max: [row.width + width, row_top + height],
      // };
      // let g = outlined.glyph();

      // // add the glyph to the row
      // row.items.push(GlyphTexInfo {
      //   glyph_info,
      //   tex_coords: unaligned_tex_coords,
      //   bounds_minus_position_over_scale: Rect {
      //     min: point(
      //       (bounds.min.x - g.position.x) / g.scale.x,
      //       (bounds.min.y - g.position.y) / g.scale.y,
      //     ),
      //     max: point(
      //       (bounds.max.x - g.position.x) / g.scale.x,
      //       (bounds.max.y - g.position.y) / g.scale.y,
      //     ),
      //   },
      // });
      // row.width += width;
      // in_use_rows.insert(row_top);

      // draw_and_upload.push((aligned_tex_coords, outlined));

      // self
      //   .all_glyphs
      //   .insert(glyph_info, (row_top, row.glyphs.len() as u32 - 1));
    } else {
      // See if there is space for a new row
      let gap = self
        .space_end_for_start
        .iter()
        .find(|&(start, end)| end - start >= height)
        .map(|(&start, &end)| (start, end));

      if let Some((gap_start, gap_end)) = gap {
        // fill space for new row
        let new_space_start = gap_start + width;
        self.space_end_for_start.remove(&gap_start);
        if new_space_start == gap_end {
          self.space_start_for_end.remove(&gap_end);
        } else {
          self.space_end_for_start.insert(new_space_start, gap_end);
          self.space_start_for_end.insert(gap_end, new_space_start);
        }
        // add the row
        self.rows.insert(
          gap_start,
          Row {
            width: 0,
            height,
            items: Vec::new(),
          },
        );
        row_top = Some(gap_start);
      } else {
        // Remove old rows until room is available
        // while !self.rows.is_empty() {
        //   // check that the oldest row isn't also in use
        //   if !in_use_rows.contains(self.rows.front().unwrap().0) {
        //     // Remove row
        //     let (top, row) = self.rows.pop_front().unwrap();

        //     for g in row.glyphs {
        //       self.all_glyphs.remove(&g.glyph_info);
        //     }

        //     let (mut new_start, mut new_end) = (top, top + row.height);
        //     // Update the free space maps
        //     // Combine with neighbouring free space if possible
        //     if let Some(end) = self.space_end_for_start.remove(&new_end) {
        //       new_end = end;
        //     }
        //     if let Some(start) = self.space_start_for_end.remove(&new_start) {
        //       new_start = start;
        //     }
        //     self.space_start_for_end.insert(new_end, new_start);
        //     self.space_end_for_start.insert(new_start, new_end);
        //     if new_end - new_start >= aligned_height {
        //       // The newly formed gap is big enough
        //       gap = Some((new_start, new_end));
        //       break;
        //     }
        //   }
        //   // all rows left are in use
        //   // try a clean insert of all needed glyphs
        //   // if that doesn't work, fail
        //   else if from_empty {
        //     // already trying a clean insert, don't do it again
        //     return Err(CacheWriteErr::NoRoomForWholeQueue);
        //   } else {
        //     // signal that a retry is needed
        //     queue_success = false;
        //     break 'per_glyph;
        //   }
        // }
      }
    }

    todo!()
  }

  fn un_pack(&mut self, id: PackId) {
    todo!()
  }
}
