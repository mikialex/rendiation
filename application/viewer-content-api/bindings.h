#ifndef RENDIATION_C_HEADER
#define RENDIATION_C_HEADER

#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

/// The largest number that can be returned by [`Self::target_pixel_byte_cost`].
constexpr static const uint32_t TextureFormat_MAX_TARGET_PIXEL_BYTE_COST = 16;

enum class ToneMapType {
  None,
  Linear,
  Reinhard,
  Cineon,
  ACESFilmic,
};

/// ASTC block dimensions
enum class AstcBlock {
  /// 4x4 block compressed texture. 16 bytes per block (8 bit/px).
  B4x4,
  /// 5x4 block compressed texture. 16 bytes per block (6.4 bit/px).
  B5x4,
  /// 5x5 block compressed texture. 16 bytes per block (5.12 bit/px).
  B5x5,
  /// 6x5 block compressed texture. 16 bytes per block (4.27 bit/px).
  B6x5,
  /// 6x6 block compressed texture. 16 bytes per block (3.56 bit/px).
  B6x6,
  /// 8x5 block compressed texture. 16 bytes per block (3.2 bit/px).
  B8x5,
  /// 8x6 block compressed texture. 16 bytes per block (2.67 bit/px).
  B8x6,
  /// 8x8 block compressed texture. 16 bytes per block (2 bit/px).
  B8x8,
  /// 10x5 block compressed texture. 16 bytes per block (2.56 bit/px).
  B10x5,
  /// 10x6 block compressed texture. 16 bytes per block (2.13 bit/px).
  B10x6,
  /// 10x8 block compressed texture. 16 bytes per block (1.6 bit/px).
  B10x8,
  /// 10x10 block compressed texture. 16 bytes per block (1.28 bit/px).
  B10x10,
  /// 12x10 block compressed texture. 16 bytes per block (1.07 bit/px).
  B12x10,
  /// 12x12 block compressed texture. 16 bytes per block (0.89 bit/px).
  B12x12,
};

/// ASTC RGBA channel
enum class AstcChannel {
  /// 8 bit integer RGBA, [0, 255] converted to/from linear-color float [0, 1] in shader.
  ///
  /// [`Features::TEXTURE_COMPRESSION_ASTC`] must be enabled to use this channel.
  Unorm,
  /// 8 bit integer RGBA, Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
  ///
  /// [`Features::TEXTURE_COMPRESSION_ASTC`] must be enabled to use this channel.
  UnormSrgb,
  /// floating-point RGBA, linear-color float can be outside of the [0, 1] range.
  ///
  /// [`Features::TEXTURE_COMPRESSION_ASTC_HDR`] must be enabled to use this channel.
  Hdr,
};

/// Primitive type the input mesh is composed of.
enum class MeshPrimitiveTopology {
  /// Vertex data is a list of points. Each vertex is a new point.
  PointList = 0,
  /// Vertex data is a list of lines. Each pair of vertices composes a new line.
  ///
  /// Vertices `0 1 2 3` create two lines `0 1` and `2 3`
  LineList = 1,
  /// Vertex data is a strip of lines. Each set of two adjacent vertices form a line.
  ///
  /// Vertices `0 1 2 3` create three lines `0 1`, `1 2`, and `2 3`.
  LineStrip = 2,
  /// Vertex data is a list of triangles. Each set of 3 vertices composes a new triangle.
  ///
  /// Vertices `0 1 2 3 4 5` create two triangles `0 1 2` and `3 4 5`
  TriangleList = 3,
  /// Vertex data is a triangle strip. Each set of three adjacent vertices form a triangle.
  ///
  /// Vertices `0 1 2 3 4 5` creates four triangles `0 1 2`, `2 1 3`, `2 3 4`, and `4 3 5`
  TriangleStrip = 4,
};

enum class MeshAPIDataType {
  Position,
  Normal,
  Uv,
  Indices,
};

enum class OccStyleEffectType {
  Unlit,
  Lighted,
  Zebra,
};

enum class CullMode {
  None,
  Front,
  Back,
};

enum class OccFlavorZLayer {
  BotOSD = 0,
  Default = 1,
  Top = 2,
  TopMost = 3,
  TopOSD = 4,
};

enum class TextAlignment {
  Left,
  Center,
  Right,
};

/// Nanosecond timestamp used by the presentation engine.
///
/// The specific clock depends on the window system integration (WSI) API used.
///
/// <table>
/// <tr>
///     <td>WSI</td>
///     <td>Clock</td>
/// </tr>
/// <tr>
///     <td>IDXGISwapchain</td>
///     <td><a href="https://docs.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter">QueryPerformanceCounter</a></td>
/// </tr>
/// <tr>
///     <td>IPresentationManager</td>
///     <td><a href="https://docs.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryinterrupttimeprecise">QueryInterruptTimePrecise</a></td>
/// </tr>
/// <tr>
///     <td>CAMetalLayer</td>
///     <td><a href="https://developer.apple.com/documentation/kernel/1462446-mach_absolute_time">mach_absolute_time</a></td>
/// </tr>
/// <tr>
///     <td>VK_GOOGLE_display_timing</td>
///     <td><a href="https://linux.die.net/man/3/clock_gettime">clock_gettime(CLOCK_MONOTONIC)</a></td>
/// </tr>
/// </table>
struct PresentationTimestamp;

struct ViewerAPI;

struct ViewerQueryAPI;

struct ViewerRayPickListResult;

struct ViewerRayPickRangeResult;

struct ViewerWorldDeriveQueryAPI;

struct ViewerEntityHandle {
  uint32_t index;
  uint64_t generation;
};

struct ViewerRayPickRangeResultInfo {
  uintptr_t len;
  const ViewerEntityHandle *ptr;
};

struct ViewerRayPickResult {
  uint32_t primitive_index;
  /// in world space. the logic hit result(maybe not exactly the ray hit point if the primitive is line or points)
  float hit_position[3];
  ViewerEntityHandle scene_model_handle;
};

struct ViewerRayPickListResultInfo {
  uintptr_t len;
  const ViewerRayPickResult *ptr;
  double camera_position_world[3];
};

/// Format in which a texture’s texels are stored in GPU memory.
///
/// Certain formats additionally specify a conversion.
/// When these formats are used in a shader, the conversion automatically takes place when loading
/// from or storing to the texture.
///
/// * `Unorm` formats linearly scale the integer range of the storage format to a floating-point
///   range of 0 to 1, inclusive.
/// * `Snorm` formats linearly scale the integer range of the storage format to a floating-point
///   range of &minus;1 to 1, inclusive, except that the most negative value
///   (&minus;128 for 8-bit, &minus;32768 for 16-bit) is excluded; on conversion,
///   it is treated as identical to the second most negative
///   (&minus;127 for 8-bit, &minus;32767 for 16-bit),
///   so that the positive and negative ranges are symmetric.
/// * `UnormSrgb` formats apply the [sRGB transfer function] so that the storage is sRGB encoded
///   while the shader works with linear intensity values.
/// * `Uint`, `Sint`, and `Float` formats perform no conversion.
///
/// Corresponds to [WebGPU `GPUTextureFormat`](
/// https://gpuweb.github.io/gpuweb/#enumdef-gputextureformat).
///
/// [sRGB transfer function]: https://en.wikipedia.org/wiki/SRGB#Transfer_function_(%22gamma%22)
struct TextureFormat {
  enum class Tag {
    /// Red channel only. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    R8Unorm,
    /// Red channel only. 8 bit integer per channel. [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    R8Snorm,
    /// Red channel only. 8 bit integer per channel. Unsigned in shader.
    R8Uint,
    /// Red channel only. 8 bit integer per channel. Signed in shader.
    R8Sint,
    /// Red channel only. 16 bit integer per channel. Unsigned in shader.
    R16Uint,
    /// Red channel only. 16 bit integer per channel. Signed in shader.
    R16Sint,
    /// Red channel only. 16 bit integer per channel. [0, 65535] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_FORMAT_16BIT_NORM`] must be enabled to use this texture format.
    R16Unorm,
    /// Red channel only. 16 bit integer per channel. [&minus;32767, 32767] converted to/from float [&minus;1, 1] in shader.
    ///
    /// [`Features::TEXTURE_FORMAT_16BIT_NORM`] must be enabled to use this texture format.
    R16Snorm,
    /// Red channel only. 16 bit float per channel. Float in shader.
    R16Float,
    /// Red and green channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Rg8Unorm,
    /// Red and green channels. 8 bit integer per channel. [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    Rg8Snorm,
    /// Red and green channels. 8 bit integer per channel. Unsigned in shader.
    Rg8Uint,
    /// Red and green channels. 8 bit integer per channel. Signed in shader.
    Rg8Sint,
    /// Red channel only. 32 bit integer per channel. Unsigned in shader.
    R32Uint,
    /// Red channel only. 32 bit integer per channel. Signed in shader.
    R32Sint,
    /// Red channel only. 32 bit float per channel. Float in shader.
    R32Float,
    /// Red and green channels. 16 bit integer per channel. Unsigned in shader.
    Rg16Uint,
    /// Red and green channels. 16 bit integer per channel. Signed in shader.
    Rg16Sint,
    /// Red and green channels. 16 bit integer per channel. [0, 65535] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_FORMAT_16BIT_NORM`] must be enabled to use this texture format.
    Rg16Unorm,
    /// Red and green channels. 16 bit integer per channel. [&minus;32767, 32767] converted to/from float [&minus;1, 1] in shader.
    ///
    /// [`Features::TEXTURE_FORMAT_16BIT_NORM`] must be enabled to use this texture format.
    Rg16Snorm,
    /// Red and green channels. 16 bit float per channel. Float in shader.
    Rg16Float,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Rgba8Unorm,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    Rgba8UnormSrgb,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    Rgba8Snorm,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Unsigned in shader.
    Rgba8Uint,
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Signed in shader.
    Rgba8Sint,
    /// Blue, green, red, and alpha channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Bgra8Unorm,
    /// Blue, green, red, and alpha channels. 8 bit integer per channel. Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    Bgra8UnormSrgb,
    /// Packed unsigned float with 9 bits mantisa for each RGB component, then a common 5 bits exponent
    Rgb9e5Ufloat,
    /// Red, green, blue, and alpha channels. 10 bit integer for RGB channels, 2 bit integer for alpha channel. Unsigned in shader.
    Rgb10a2Uint,
    /// Red, green, blue, and alpha channels. 10 bit integer for RGB channels, 2 bit integer for alpha channel. [0, 1023] ([0, 3] for alpha) converted to/from float [0, 1] in shader.
    Rgb10a2Unorm,
    /// Red, green, and blue channels. 11 bit float with no sign bit for RG channels. 10 bit float with no sign bit for blue channel. Float in shader.
    Rg11b10Ufloat,
    /// Red channel only. 64 bit integer per channel. Unsigned in shader.
    ///
    /// [`Features::TEXTURE_INT64_ATOMIC`] must be enabled to use this texture format.
    R64Uint,
    /// Red and green channels. 32 bit integer per channel. Unsigned in shader.
    Rg32Uint,
    /// Red and green channels. 32 bit integer per channel. Signed in shader.
    Rg32Sint,
    /// Red and green channels. 32 bit float per channel. Float in shader.
    Rg32Float,
    /// Red, green, blue, and alpha channels. 16 bit integer per channel. Unsigned in shader.
    Rgba16Uint,
    /// Red, green, blue, and alpha channels. 16 bit integer per channel. Signed in shader.
    Rgba16Sint,
    /// Red, green, blue, and alpha channels. 16 bit integer per channel. [0, 65535] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_FORMAT_16BIT_NORM`] must be enabled to use this texture format.
    Rgba16Unorm,
    /// Red, green, blue, and alpha. 16 bit integer per channel. [&minus;32767, 32767] converted to/from float [&minus;1, 1] in shader.
    ///
    /// [`Features::TEXTURE_FORMAT_16BIT_NORM`] must be enabled to use this texture format.
    Rgba16Snorm,
    /// Red, green, blue, and alpha channels. 16 bit float per channel. Float in shader.
    Rgba16Float,
    /// Red, green, blue, and alpha channels. 32 bit integer per channel. Unsigned in shader.
    Rgba32Uint,
    /// Red, green, blue, and alpha channels. 32 bit integer per channel. Signed in shader.
    Rgba32Sint,
    /// Red, green, blue, and alpha channels. 32 bit float per channel. Float in shader.
    Rgba32Float,
    /// Stencil format with 8 bit integer stencil.
    Stencil8,
    /// Special depth format with 16 bit integer depth.
    Depth16Unorm,
    /// Special depth format with at least 24 bit integer depth.
    Depth24Plus,
    /// Special depth/stencil format with at least 24 bit integer depth and 8 bits integer stencil.
    Depth24PlusStencil8,
    /// Special depth format with 32 bit floating point depth.
    Depth32Float,
    /// Special depth/stencil format with 32 bit floating point depth and 8 bits integer stencil.
    ///
    /// [`Features::DEPTH32FLOAT_STENCIL8`] must be enabled to use this texture format.
    Depth32FloatStencil8,
    /// YUV 4:2:0 chroma subsampled format.
    ///
    /// Contains two planes:
    /// - 0: Single 8 bit channel luminance.
    /// - 1: Dual 8 bit channel chrominance at half width and half height.
    ///
    /// Valid view formats for luminance are [`TextureFormat::R8Unorm`].
    ///
    /// Valid view formats for chrominance are [`TextureFormat::Rg8Unorm`].
    ///
    /// Width and height must be even.
    ///
    /// [`Features::TEXTURE_FORMAT_NV12`] must be enabled to use this texture format.
    NV12,
    /// YUV 4:2:0 chroma subsampled format.
    ///
    /// Contains two planes:
    /// - 0: Single 16 bit channel luminance, of which only the high 10 bits
    ///   are used.
    /// - 1: Dual 16 bit channel chrominance at half width and half height, of
    ///   which only the high 10 bits are used.
    ///
    /// Valid view formats for luminance are [`TextureFormat::R16Unorm`].
    ///
    /// Valid view formats for chrominance are [`TextureFormat::Rg16Unorm`].
    ///
    /// Width and height must be even.
    ///
    /// [`Features::TEXTURE_FORMAT_P010`] must be enabled to use this texture format.
    P010,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). 4 color + alpha pallet. 5 bit R + 6 bit G + 5 bit B + 1 bit alpha.
    /// [0, 63] ([0, 1] for alpha) converted to/from float [0, 1] in shader.
    ///
    /// Also known as DXT1.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc1RgbaUnorm,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). 4 color + alpha pallet. 5 bit R + 6 bit G + 5 bit B + 1 bit alpha.
    /// Srgb-color [0, 63] ([0, 1] for alpha) converted to/from linear-color float [0, 1] in shader.
    ///
    /// Also known as DXT1.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc1RgbaUnormSrgb,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). 4 color pallet. 5 bit R + 6 bit G + 5 bit B + 4 bit alpha.
    /// [0, 63] ([0, 15] for alpha) converted to/from float [0, 1] in shader.
    ///
    /// Also known as DXT3.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc2RgbaUnorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). 4 color pallet. 5 bit R + 6 bit G + 5 bit B + 4 bit alpha.
    /// Srgb-color [0, 63] ([0, 255] for alpha) converted to/from linear-color float [0, 1] in shader.
    ///
    /// Also known as DXT3.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc2RgbaUnormSrgb,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). 4 color pallet + 8 alpha pallet. 5 bit R + 6 bit G + 5 bit B + 8 bit alpha.
    /// [0, 63] ([0, 255] for alpha) converted to/from float [0, 1] in shader.
    ///
    /// Also known as DXT5.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc3RgbaUnorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). 4 color pallet + 8 alpha pallet. 5 bit R + 6 bit G + 5 bit B + 8 bit alpha.
    /// Srgb-color [0, 63] ([0, 255] for alpha) converted to/from linear-color float [0, 1] in shader.
    ///
    /// Also known as DXT5.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc3RgbaUnormSrgb,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). 8 color pallet. 8 bit R.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// Also known as RGTC1.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc4RUnorm,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). 8 color pallet. 8 bit R.
    /// [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    ///
    /// Also known as RGTC1.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc4RSnorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). 8 color red pallet + 8 color green pallet. 8 bit RG.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// Also known as RGTC2.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc5RgUnorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). 8 color red pallet + 8 color green pallet. 8 bit RG.
    /// [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    ///
    /// Also known as RGTC2.
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc5RgSnorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Variable sized pallet. 16 bit unsigned float RGB. Float in shader.
    ///
    /// Also known as BPTC (float).
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc6hRgbUfloat,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Variable sized pallet. 16 bit signed float RGB. Float in shader.
    ///
    /// Also known as BPTC (float).
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc6hRgbFloat,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Variable sized pallet. 8 bit integer RGBA.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// Also known as BPTC (unorm).
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc7RgbaUnorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Variable sized pallet. 8 bit integer RGBA.
    /// Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    ///
    /// Also known as BPTC (unorm).
    ///
    /// [`Features::TEXTURE_COMPRESSION_BC`] must be enabled to use this texture format.
    /// [`Features::TEXTURE_COMPRESSION_BC_SLICED_3D`] must be enabled to use this texture format with 3D dimension.
    Bc7RgbaUnormSrgb,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). Complex pallet. 8 bit integer RGB.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    Etc2Rgb8Unorm,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). Complex pallet. 8 bit integer RGB.
    /// Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    Etc2Rgb8UnormSrgb,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). Complex pallet. 8 bit integer RGB + 1 bit alpha.
    /// [0, 255] ([0, 1] for alpha) converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    Etc2Rgb8A1Unorm,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). Complex pallet. 8 bit integer RGB + 1 bit alpha.
    /// Srgb-color [0, 255] ([0, 1] for alpha) converted to/from linear-color float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    Etc2Rgb8A1UnormSrgb,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Complex pallet. 8 bit integer RGB + 8 bit alpha.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    Etc2Rgba8Unorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Complex pallet. 8 bit integer RGB + 8 bit alpha.
    /// Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    Etc2Rgba8UnormSrgb,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). Complex pallet. 11 bit integer R.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    EacR11Unorm,
    /// 4x4 block compressed texture. 8 bytes per block (4 bit/px). Complex pallet. 11 bit integer R.
    /// [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    EacR11Snorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Complex pallet. 11 bit integer R + 11 bit integer G.
    /// [0, 255] converted to/from float [0, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    EacRg11Unorm,
    /// 4x4 block compressed texture. 16 bytes per block (8 bit/px). Complex pallet. 11 bit integer R + 11 bit integer G.
    /// [&minus;127, 127] converted to/from float [&minus;1, 1] in shader.
    ///
    /// [`Features::TEXTURE_COMPRESSION_ETC2`] must be enabled to use this texture format.
    EacRg11Snorm,
    /// block compressed texture. 16 bytes per block.
    ///
    /// Features [`TEXTURE_COMPRESSION_ASTC`] or [`TEXTURE_COMPRESSION_ASTC_HDR`]
    /// must be enabled to use this texture format.
    ///
    /// [`TEXTURE_COMPRESSION_ASTC`]: Features::TEXTURE_COMPRESSION_ASTC
    /// [`TEXTURE_COMPRESSION_ASTC_HDR`]: Features::TEXTURE_COMPRESSION_ASTC_HDR
    Astc,
  };

  struct Astc_Body {
    /// compressed block dimensions
    AstcBlock block;
    /// ASTC RGBA channel
    AstcChannel channel;
  };

  Tag tag;
  union {
    Astc_Body astc;
  };
};

struct VertexPair {
  ViewerEntityHandle h1;
  ViewerEntityHandle h2;
};

struct AttributesMeshEntitiesCommon {
  ViewerEntityHandle mesh;
  ViewerEntityHandle index;
  VertexPair position;
  VertexPair normal;
  bool has_normal;
  VertexPair uv;
  bool has_uv;
};

struct OccControlStateSimple {
  bool enable_depth_test;
  bool enable_depth_write;
  bool front_face_ccw;
  float depth_bias_constant_factor;
  float depth_bias_slop_factor;
  float depth_bias_clamp;
  bool enable_alpha_blend;
  CullMode cull_mode;
};

struct SceneModelHandleInfo {
  ViewerEntityHandle scene_model;
  ViewerEntityHandle std_model;
};

struct SceneWidePointsHandleInfo {
  ViewerEntityHandle scene_model;
  ViewerEntityHandle points;
};

struct SceneWideLineHandleInfo {
  ViewerEntityHandle scene_model;
  ViewerEntityHandle line;
};

struct SceneText3dHandleInfo {
  ViewerEntityHandle scene_model;
  ViewerEntityHandle text3d;
};

struct Text3dContentInfoC {
  const char *content;
  float font_size;
  float line_height;
  float scale;
  const char *font;
  uint32_t weight;
  bool has_weight;
  float color[4];
  bool italic;
  float width;
  bool has_width;
  float height;
  bool has_height;
  TextAlignment align;
};



extern "C" {

ViewerEntityHandle create_camera(ViewerEntityHandle node);

void drop_camera(ViewerEntityHandle handle);

void camera_set_lookat_position(ViewerEntityHandle handle, const float (*position)[3]);

void camera_set_proj_perspective(ViewerEntityHandle handle,
                                 float near,
                                 float far,
                                 float vertical_fov_in_deg,
                                 float aspect);

void camera_set_proj_orth(ViewerEntityHandle handle,
                          float near,
                          float far,
                          float left,
                          float right,
                          float top,
                          float bottom);

ViewerEntityHandle create_node();

void delete_node(ViewerEntityHandle node);

void node_set_local_mat(ViewerEntityHandle node, const double (*mat4)[16]);

/// set parent to null_ptr to detach
void node_attach_parent(ViewerEntityHandle node, ViewerEntityHandle *parent);

ViewerAPI *create_viewer_content_api_instance(const char *config_path);

void drop_viewer_content_api_instance(ViewerAPI *api);

void viewer_set_tonemap_ty_value(ViewerAPI *api, ToneMapType ty, float exposure);

/// hinstance can be null_ptr
uint32_t viewer_create_surface(ViewerAPI *api,
                               void *hwnd,
                               void *hinstance,
                               uint32_t width,
                               uint32_t height);

void viewer_drop_surface(ViewerAPI *api, uint32_t surface_id);

void viewer_surface_set_camera(ViewerAPI *api, uint32_t surface_id, ViewerEntityHandle camera);

void viewer_surface_set_scene(ViewerAPI *api, uint32_t surface_id, ViewerEntityHandle scene);

/// may return empty handle for error case
ViewerEntityHandle viewer_read_last_render_result(ViewerAPI *api, uint32_t surface_id);

/// the size is physical resolution
void viewer_resize(ViewerAPI *api, uint32_t surface_id, uint32_t new_width, uint32_t new_height);

void viewer_load_font(ViewerAPI *api, const char *font_path);

void viewer_render_surface(ViewerAPI *api, uint32_t surface_id);

ViewerWorldDeriveQueryAPI *viewer_create_world_derive_query_api(ViewerAPI *api);

/// api must be dropped before any scene related modifications, or deadlock will occur
void viewer_drop_world_derive_query_api(ViewerWorldDeriveQueryAPI *api);

bool world_derive_query_api_get_world_mat(ViewerWorldDeriveQueryAPI *api,
                                          ViewerEntityHandle node,
                                          double (*r)[16]);

bool world_derive_query_api_get_world_bounding(ViewerWorldDeriveQueryAPI *api,
                                               ViewerEntityHandle sm,
                                               double (*result)[6]);

bool world_derive_query_api_get_local_bounding(ViewerWorldDeriveQueryAPI *api,
                                               ViewerEntityHandle sm,
                                               float (*result)[6]);

ViewerQueryAPI *viewer_create_picker_api(ViewerAPI *api, uint32_t surface_id);

/// api must be dropped before any scene related modifications, or deadlock will occur
void viewer_drop_picker_api(ViewerQueryAPI *api);

void query_scene_bounding(ViewerWorldDeriveQueryAPI *api,
                          ViewerAPI *viewer_api,
                          ViewerEntityHandle scene,
                          float (*result)[6],
                          bool consider_override,
                          uint32_t surface_id);

/// the returned pick list's should be dropped by  [drop_pick_list_result] after read the result
///
/// all inputs are logic pixel
ViewerRayPickListResult *picker_pick_list(ViewerQueryAPI *api,
                                          ViewerAPI *viewer,
                                          ViewerEntityHandle scene,
                                          float x,
                                          float y,
                                          float extra_screen_space_tolerance);

void drop_pick_list_result(ViewerRayPickListResult *r);

/// the returned pick range's should be dropped by  [drop_pick_range_result] after read the result
///
/// the a, b point can be swapped without order limits.
///
/// all inputs are logic pixel
ViewerRayPickRangeResult *picker_pick_range(ViewerQueryAPI *api,
                                            ViewerAPI *viewer,
                                            ViewerEntityHandle scene,
                                            float ax,
                                            float ay,
                                            float bx,
                                            float by,
                                            bool contains,
                                            bool precise_intersection_test,
                                            float extra_screen_space_tolerance);

void drop_pick_range_result(ViewerRayPickRangeResult *r);

ViewerRayPickRangeResultInfo get_ray_pick_range_info(ViewerRayPickRangeResult *r);

ViewerRayPickListResultInfo get_ray_pick_list_info(ViewerRayPickListResult *r);

ViewerEntityHandle create_scene();

void drop_scene(ViewerEntityHandle handle);

void scene_set_background_solid(ViewerEntityHandle handle, const float (*color)[3]);

void scene_set_background_gradient(ViewerEntityHandle handle,
                                   const float (*top)[3],
                                   const float (*bottom)[3]);

/// the content format expects Rgba8UnormSrgb
ViewerEntityHandle create_texture2d(const uint8_t *content,
                                    uintptr_t len,
                                    uint32_t width,
                                    uint32_t height,
                                    TextureFormat format);

void update_texture2d_content(ViewerEntityHandle handle,
                              const uint8_t *content,
                              uintptr_t len,
                              uint32_t width,
                              uint32_t height,
                              TextureFormat format);

ViewerEntityHandle create_texture_cube();

void drop_texture_cube(ViewerEntityHandle handle);

void texture_cube_set_face(ViewerEntityHandle cube, uint32_t face_index, ViewerEntityHandle tex);

void drop_texture2d(ViewerEntityHandle handle);

ViewerEntityHandle create_sampler();

void drop_sampler(ViewerEntityHandle handle);

AttributesMeshEntitiesCommon create_mesh(uint32_t indices_length,
                                         const uint32_t *indices,
                                         uint32_t vertex_length,
                                         const float *position,
                                         const float *normal_raw,
                                         const float *uv_raw,
                                         MeshPrimitiveTopology topo);

void drop_mesh(AttributesMeshEntitiesCommon entities);

void update_mesh_data(AttributesMeshEntitiesCommon *entities,
                      uint32_t byte_size,
                      const uint8_t *data,
                      MeshAPIDataType vertex_ty);

void set_mesh_topology(ViewerEntityHandle mesh, MeshPrimitiveTopology topo);

ViewerEntityHandle create_occ_material();

void drop_occ_material(ViewerEntityHandle handle);

void occ_material_set_transparent(ViewerEntityHandle mat, bool transparent);

void occ_material_set_diffuse(ViewerEntityHandle mat, const float (*color)[4]);

void occ_material_set_specular(ViewerEntityHandle mat, const float (*color)[3]);

void occ_material_set_shininess(ViewerEntityHandle mat, float shininess);

void occ_material_set_emissive(ViewerEntityHandle mat, const float (*color)[3]);

ViewerEntityHandle create_occ_effect_control();

void drop_occ_effect_control(ViewerEntityHandle handle);

void occ_material_set_effect(ViewerEntityHandle mat, ViewerEntityHandle effect);

void occ_effect_control_set_shade_type(ViewerEntityHandle effect, OccStyleEffectType shade_type);

void occ_effect_control_set_state(ViewerEntityHandle effect, OccControlStateSimple simple_config);

void occ_material_set_diffuse_tex(ViewerEntityHandle mat,
                                  ViewerEntityHandle tex,
                                  ViewerEntityHandle sampler);

void std_model_set_occ_material(ViewerEntityHandle handle, ViewerEntityHandle material);

ViewerEntityHandle create_unlit_material();

void unlit_material_set_color(ViewerEntityHandle mat, const float (*color)[4]);

void drop_unlit_material(ViewerEntityHandle handle);

ViewerEntityHandle create_pbr_mr_material();

void pbr_mr_material_set_base_color(ViewerEntityHandle mat, const float (*color)[3]);

void pbr_mr_material_set_base_color_tex(ViewerEntityHandle mat,
                                        ViewerEntityHandle tex,
                                        ViewerEntityHandle sampler);

void drop_pbr_mr_material(ViewerEntityHandle handle);

SceneModelHandleInfo create_scene_model(ViewerEntityHandle material,
                                        ViewerEntityHandle mesh,
                                        ViewerEntityHandle node,
                                        ViewerEntityHandle scene);

void drop_scene_model(SceneModelHandleInfo handle);

void scene_model_set_visible(ViewerEntityHandle handle, bool visible);

void scene_model_set_mesh(SceneModelHandleInfo handle, ViewerEntityHandle mesh);

void scene_model_set_scene(ViewerEntityHandle handle, const ViewerEntityHandle *scene);

void scene_model_set_occ_style_view_dep(ViewerEntityHandle handle,
                                        bool is_2d,
                                        const float (*anchor)[3],
                                        const int32_t (*offset)[2],
                                        uint32_t corner,
                                        uint32_t mode);

void scene_model_remove_occ_style_view_dep(ViewerEntityHandle handle);

void scene_model_set_z_layer(ViewerEntityHandle handle, OccFlavorZLayer z_layer);

void scene_model_set_priority(ViewerEntityHandle handle, uint32_t priority);

void scene_model_set_selectable(ViewerEntityHandle handle, bool selectable);

void scene_model_set_material(SceneModelHandleInfo handle, ViewerEntityHandle material);

SceneWidePointsHandleInfo create_wide_points(ViewerEntityHandle node,
                                             uint32_t data_length,
                                             const uint8_t *data);

void wide_points_set_buffer(ViewerEntityHandle handle, uint32_t data_length, const uint8_t *data);

void wide_points_set_color(ViewerEntityHandle handle, const float (*color)[4]);

void wide_points_set_depth_test(ViewerEntityHandle handle, bool bool_);

void wide_points_set_pattern_texture(ViewerEntityHandle handle,
                                     ViewerEntityHandle texture,
                                     ViewerEntityHandle sampler);

void drop_wide_points(SceneWidePointsHandleInfo p);

SceneWideLineHandleInfo create_wide_line(ViewerEntityHandle node,
                                         uint32_t data_length,
                                         const uint8_t *data);

void wide_line_set_buffer(ViewerEntityHandle handle, uint32_t data_length, const uint8_t *data);

void wide_line_set_enable_depth_test(ViewerEntityHandle handle, bool enabled);

void wide_line_set_color(ViewerEntityHandle handle, const float (*color)[4]);

void wide_line_set_width(ViewerEntityHandle handle, const float *width);

void wide_line_set_pattern(ViewerEntityHandle handle, uint32_t pattern);

void wide_line_set_factor(ViewerEntityHandle handle, float factor);

void drop_wide_line(SceneWideLineHandleInfo p);

SceneText3dHandleInfo create_text3d(ViewerEntityHandle node, const Text3dContentInfoC *content);

void text3d_set_content(ViewerEntityHandle handle, const Text3dContentInfoC *content);

void drop_text3d(SceneText3dHandleInfo p);

ViewerEntityHandle create_dir_light(ViewerEntityHandle node);

void set_dir_light_scene(ViewerEntityHandle handle, const ViewerEntityHandle *scene);

void set_dir_light_follow_camera(ViewerEntityHandle node, bool should_follow);

void set_dir_light_illuminance(ViewerEntityHandle node, const float (*illuminance)[3]);

void drop_dir_light(ViewerEntityHandle handle);

ViewerEntityHandle create_point_light(ViewerEntityHandle node);

void set_point_light_scene(ViewerEntityHandle handle, const ViewerEntityHandle *scene);

void set_point_light_intensity(ViewerEntityHandle node, const float (*illuminance)[3]);

void set_point_light_cutoff_distance(ViewerEntityHandle node, float distance);

void drop_point_light(ViewerEntityHandle handle);

ViewerEntityHandle create_spot_light(ViewerEntityHandle node);

void set_spot_light_scene(ViewerEntityHandle handle, const ViewerEntityHandle *scene);

void set_spot_light_intensity(ViewerEntityHandle node, const float (*illuminance)[3]);

void set_spot_light_cutoff_distance(ViewerEntityHandle node, float distance);

void set_spot_light_half_cone_angle(ViewerEntityHandle node, float angle);

void set_spot_light_half_penumbra_angle(ViewerEntityHandle node, float angle);

void drop_spot_light(ViewerEntityHandle handle);

ViewerEntityHandle create_clipping_plane(const float (*plane)[4], const ViewerEntityHandle *scene);

void drop_clipping_plane(ViewerEntityHandle handle);

void clipping_plane_set_plane(ViewerEntityHandle handle, const float (*plane)[4]);

void clipping_plane_set_scene(ViewerEntityHandle handle, const ViewerEntityHandle *scene);

void attribute_mesh_set_is_solid(ViewerEntityHandle handle, bool is_solid);

/// call this to setup panic message writer when panic happens
void rendiation_init();

}  // extern "C"

#endif  // RENDIATION_C_HEADER
