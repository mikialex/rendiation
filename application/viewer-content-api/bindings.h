#ifndef RENDIATION_C_HEADER
#define RENDIATION_C_HEADER

#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

enum class ToneMapType {
  None,
  Linear,
  Reinhard,
  Cineon,
  ACESFilmic,
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

enum class OccStyleEffectType {
  Unlit,
  Lighted,
  Zebra,
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

void viewer_load_font(ViewerAPI *api, uint32_t data_length, const uint8_t *data);

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
                                          float y);

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
                                            bool contains);

void drop_pick_range_result(ViewerRayPickRangeResult *r);

ViewerRayPickRangeResultInfo get_ray_pick_range_info(ViewerRayPickRangeResult *r);

ViewerRayPickListResultInfo get_ray_pick_list_info(ViewerRayPickListResult *r);

ViewerEntityHandle create_scene();

void drop_scene(ViewerEntityHandle handle);

/// the content format expects Rgba8UnormSrgb
ViewerEntityHandle create_texture2d(const uint8_t *content,
                                    uintptr_t len,
                                    uint32_t width,
                                    uint32_t height);

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

void wide_points_set_pattern_texture(ViewerEntityHandle handle,
                                     ViewerEntityHandle texture,
                                     ViewerEntityHandle sampler);

void drop_wide_points(SceneWidePointsHandleInfo p);

SceneWideLineHandleInfo create_wide_line(ViewerEntityHandle node,
                                         uint32_t data_length,
                                         const uint8_t *data);

void wide_line_set_buffer(ViewerEntityHandle handle, uint32_t data_length, const uint8_t *data);

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
