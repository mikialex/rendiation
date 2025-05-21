use core::str;
use std::borrow::Cow;

use rendiation_texture_gpu_base::create_gpu_texture2d;

use crate::*;

pub fn create_gpu_tex_from_png_buffer(
  cx: &GPU,
  buf: &[u8],
  format: TextureFormat,
) -> GPU2DTextureView {
  let png_decoder = png::Decoder::new(buf);
  let mut png_reader = png_decoder.read_info().unwrap();
  let mut buf = vec![0; png_reader.output_buffer_size()];
  png_reader.next_frame(&mut buf).unwrap();

  let (width, height) = png_reader.info().size();
  create_gpu_texture2d(
    cx,
    &GPUBufferImage {
      data: buf,
      format,
      size: Size::from_u32_pair_min_one((width, height)),
    },
  )
}

const SPECIAL_TYPE_CHARS: [u8; 9] = *b" <>()[],;";
/// Shortens a type name to remove all module paths.
///
/// The short name of a type is its full name as returned by
/// [`std::any::type_name`], but with the prefix of all paths removed. For
/// example, the short name of `alloc::vec::Vec<core::option::Option<u32>>`
/// would be `Vec<Option<u32>>`.
pub fn get_short_name(full_name: &str) -> Cow<str> {
  // Generics result in nested paths within <..> blocks.
  // Consider "bevy_render::camera::camera::extract_cameras<bevy_render::camera::bundle::Camera3d>".
  // To tackle this, we parse the string from left to right, collapsing as we go.
  let mut remaining = full_name.as_bytes();
  let mut parsed_name = Vec::new();
  let mut complex_type = false;

  loop {
    // Collapse everything up to the next special character,
    // then skip over it
    let is_special = |c| SPECIAL_TYPE_CHARS.contains(c);
    if let Some(next_special_index) = remaining.iter().position(is_special) {
      complex_type = true;
      if parsed_name.is_empty() {
        parsed_name.reserve(remaining.len());
      }
      let (pre_special, post_special) = remaining.split_at(next_special_index + 1);
      parsed_name.extend_from_slice(collapse_type_name(pre_special));
      match pre_special.last().unwrap() {
        b'>' | b')' | b']' if post_special.get(..2) == Some(b"::") => {
          parsed_name.extend_from_slice(b"::");
          // Move the index past the "::"
          remaining = &post_special[2..];
        }
        // Move the index just past the special character
        _ => remaining = post_special,
      }
    } else if !complex_type {
      let collapsed = collapse_type_name(remaining);
      // SAFETY: We only split on ASCII characters, and the input is valid UTF8, since
      // it was a &str
      let str = unsafe { str::from_utf8_unchecked(collapsed) };
      return Cow::Borrowed(str);
    } else {
      // If there are no special characters left, we're done!
      parsed_name.extend_from_slice(collapse_type_name(remaining));
      // SAFETY: see above
      let utf8_name = unsafe { String::from_utf8_unchecked(parsed_name) };
      return Cow::Owned(utf8_name);
    }
  }
}

#[inline(always)]
fn collapse_type_name(string: &[u8]) -> &[u8] {
  let find = |(index, window)| (window == b"::").then_some(index + 2);
  let split_index = string.windows(2).enumerate().rev().find_map(find);
  &string[split_index.unwrap_or(0)..]
}
