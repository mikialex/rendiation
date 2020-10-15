pub trait RALVertexBufferDescriptorProvider {
  fn create_descriptor() -> RALVertexBufferDescriptor;
}

pub struct RALVertexBufferDescriptor {
  pub byte_stride: i32,
  pub attributes: Vec<RALVertexAttributeBufferDescriptor>,
}

pub struct RALVertexAttributeBufferDescriptor {
  pub byte_offset: i32,
  pub format: RALVertexAttributeFormat,
}

#[derive(Copy, Clone, Debug)]
pub enum RALVertexAttributeFormat {
  /// Two unsigned bytes (u8). `uvec2` in shaders.
  Uchar2 = 0,
  /// Four unsigned bytes (u8). `uvec4` in shaders.
  Uchar4 = 1,
  /// Two signed bytes (i8). `ivec2` in shaders.
  Char2 = 2,
  /// Four signed bytes (i8). `ivec4` in shaders.
  Char4 = 3,
  /// Two unsigned bytes (u8). [0, 255] converted to float [0, 1] `vec2` in shaders.
  Uchar2Norm = 4,
  /// Four unsigned bytes (u8). [0, 255] converted to float [0, 1] `vec4` in shaders.
  Uchar4Norm = 5,
  /// Two signed bytes (i8). [-127, 127] converted to float [-1, 1] `vec2` in shaders.
  Char2Norm = 6,
  /// Four signed bytes (i8). [-127, 127] converted to float [-1, 1] `vec4` in shaders.
  Char4Norm = 7,
  /// Two unsigned shorts (u16). `uvec2` in shaders.
  Ushort2 = 8,
  /// Four unsigned shorts (u16). `uvec4` in shaders.
  Ushort4 = 9,
  /// Two signed shorts (i16). `ivec2` in shaders.
  Short2 = 10,
  /// Four signed shorts (i16). `ivec4` in shaders.
  Short4 = 11,
  /// Two unsigned shorts (u16). [0, 65535] converted to float [0, 1] `vec2` in shaders.
  Ushort2Norm = 12,
  /// Four unsigned shorts (u16). [0, 65535] converted to float [0, 1] `vec4` in shaders.
  Ushort4Norm = 13,
  /// Two signed shorts (i16). [-32767, 32767] converted to float [-1, 1] `vec2` in shaders.
  Short2Norm = 14,
  /// Four signed shorts (i16). [-32767, 32767] converted to float [-1, 1] `vec4` in shaders.
  Short4Norm = 15,
  /// Two half-precision floats (no Rust equiv). `vec2` in shaders.
  Half2 = 16,
  /// Four half-precision floats (no Rust equiv). `vec4` in shaders.
  Half4 = 17,
  /// One single-precision float (f32). `float` in shaders.
  Float = 18,
  /// Two single-precision floats (f32). `vec2` in shaders.
  Float2 = 19,
  /// Three single-precision floats (f32). `vec3` in shaders.
  Float3 = 20,
  /// Four single-precision floats (f32). `vec4` in shaders.
  Float4 = 21,
  /// One unsigned int (u32). `uint` in shaders.
  Uint = 22,
  /// Two unsigned ints (u32). `uvec2` in shaders.
  Uint2 = 23,
  /// Three unsigned ints (u32). `uvec3` in shaders.
  Uint3 = 24,
  /// Four unsigned ints (u32). `uvec4` in shaders.
  Uint4 = 25,
  /// One signed int (i32). `int` in shaders.
  Int = 26,
  /// Two signed ints (i32). `ivec2` in shaders.
  Int2 = 27,
  /// Three signed ints (i32). `ivec3` in shaders.
  Int3 = 28,
  /// Four signed ints (i32). `ivec4` in shaders.
  Int4 = 29,
}
