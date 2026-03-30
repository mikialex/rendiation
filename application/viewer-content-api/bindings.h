#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Primitive type the input mesh is composed of.
 */
typedef enum MeshPrimitiveTopology {
  /**
   * Vertex data is a list of points. Each vertex is a new point.
   */
  PointList = 0,
  /**
   * Vertex data is a list of lines. Each pair of vertices composes a new line.
   *
   * Vertices `0 1 2 3` create two lines `0 1` and `2 3`
   */
  LineList = 1,
  /**
   * Vertex data is a strip of lines. Each set of two adjacent vertices form a line.
   *
   * Vertices `0 1 2 3` create three lines `0 1`, `1 2`, and `2 3`.
   */
  LineStrip = 2,
  /**
   * Vertex data is a list of triangles. Each set of 3 vertices composes a new triangle.
   *
   * Vertices `0 1 2 3 4 5` create two triangles `0 1 2` and `3 4 5`
   */
  TriangleList = 3,
  /**
   * Vertex data is a triangle strip. Each set of three adjacent vertices form a triangle.
   *
   * Vertices `0 1 2 3 4 5` creates four triangles `0 1 2`, `2 1 3`, `2 3 4`, and `4 3 5`
   */
  TriangleStrip = 4,
} MeshPrimitiveTopology;

typedef struct ViewerAPI ViewerAPI;

typedef struct ViewerPickerAPI ViewerPickerAPI;

typedef struct ViewerRayPickListResult ViewerRayPickListResult;

typedef struct ViewerEntityHandle {
  uint32_t index;
  uint64_t generation;
} ViewerEntityHandle;

typedef struct VertexPair {
  struct ViewerEntityHandle h1;
  struct ViewerEntityHandle h2;
} VertexPair;

typedef struct AttributesMeshEntitiesCommon {
  struct ViewerEntityHandle mesh;
  struct ViewerEntityHandle index;
  struct VertexPair position;
  struct VertexPair normal;
  bool has_normal;
  struct VertexPair uv;
  bool has_uv;
} AttributesMeshEntitiesCommon;

typedef struct ViewerRayPickResult {
  uint32_t primitive_index;
  /**
   * in world space. the logic hit result(maybe not exactly the ray hit point if the primitive is line or points)
   */
  float hit_position[3];
  struct ViewerEntityHandle scene_model_handle;
} ViewerRayPickResult;

typedef struct ViewerRayPickListResultInfo {
  uintptr_t len;
  const struct ViewerRayPickResult *ptr;
} ViewerRayPickListResultInfo;

struct ViewerAPI *create_viewer_content_api_instance(void);

void drop_viewer_content_api_instance(struct ViewerAPI *api);

/**
 * hinstance can be null_ptr
 */
uint32_t viewer_create_view(struct ViewerAPI *api, void *hwnd, void *hinstance);

void viewer_drop_view(struct ViewerAPI *api, uint32_t view_id);

/**
 * the size is physical resolution
 */
void viewer_resize(struct ViewerAPI *api,
                   uint32_t view_id,
                   uint32_t new_width,
                   uint32_t new_height);

struct ViewerEntityHandle create_node(void);

void delete_node(struct ViewerEntityHandle node);

void node_set_local_mat(struct ViewerEntityHandle node, const double (*mat4)[16]);

void node_attach_parent(struct ViewerEntityHandle node, struct ViewerEntityHandle *parent);

struct AttributesMeshEntitiesCommon create_mesh(uint32_t indices_length,
                                                const uint32_t *indices,
                                                uint32_t vertex_length,
                                                const float *position,
                                                const float *normal_raw,
                                                const float *uv_raw,
                                                enum MeshPrimitiveTopology topo);

void drop_mesh(struct AttributesMeshEntitiesCommon entities);

/**
 * the content format expects Rgba8UnormSrgb
 */
struct ViewerEntityHandle create_texture2d(const uint8_t *content,
                                           uintptr_t len,
                                           uint32_t width,
                                           uint32_t height);

void drop_texture2d(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_sampler(void);

void drop_sampler(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_unlit_material(void);

void unlit_material_set_color(struct ViewerEntityHandle mat, const float (*color)[4]);

void drop_unlit_material(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_pbr_mr_material(void);

void pbr_mr_material_set_color(struct ViewerEntityHandle mat, const float (*color)[3]);

void pbr_mr_material_set_color_tex(struct ViewerEntityHandle mat,
                                   struct ViewerEntityHandle tex,
                                   struct ViewerEntityHandle sampler);

void drop_pbr_mr_material(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_wide_line(void);

void drop_wide_line(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_text3d(void);

void drop_text3d(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_camera(struct ViewerEntityHandle node);

void drop_camera(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_dir_light(struct ViewerEntityHandle node);

void drop_dir_light(struct ViewerEntityHandle handle);

struct ViewerEntityHandle create_scene_model(struct ViewerEntityHandle material,
                                             struct ViewerEntityHandle mesh);

void drop_scene_model(struct ViewerEntityHandle handle);

void viewer_render(struct ViewerAPI *api);

struct ViewerPickerAPI *viewer_create_picker_api(struct ViewerAPI *api);

/**
 * picker api must be dropped before any scene related modifications, or deadlock will occur
 */
void viewer_drop_picker_api(struct ViewerPickerAPI *api);

/**
 * the returned pick list's should be dropped by  [drop_pick_list_result] after read the result
 */
struct ViewerRayPickListResult *picker_pick_list(struct ViewerPickerAPI *api,
                                                 struct ViewerAPI *viewer,
                                                 struct ViewerEntityHandle scene,
                                                 float x,
                                                 float y);

void drop_pick_list_result(struct ViewerRayPickListResult *r);

struct ViewerRayPickListResultInfo get_ray_pick_list_info(struct ViewerRayPickListResult *r);

/**
 * call this to setup panic message writer when panic happens
 */
void setup_panic_message_file_writer(void);
