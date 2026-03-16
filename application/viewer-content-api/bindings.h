#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct ViewerAPI ViewerAPI;

typedef struct ViewerPickerAPI ViewerPickerAPI;

typedef struct ViewerEntityHandle {
  uint32_t index;
  uint64_t generation;
} ViewerEntityHandle;

struct ViewerAPI *create_viewer_content_api_instance(int32_t hwnd);

void drop_viewer_content_api_instance(struct ViewerAPI *api);

void viewer_resize(struct ViewerAPI *api, uint32_t new_width, uint32_t new_height);

struct ViewerEntityHandle viewer_create_node(void);

void viewer_delete_node(struct ViewerEntityHandle node);

void viewer_node_attach_parent(struct ViewerEntityHandle node, struct ViewerEntityHandle *parent);

void viewer_render(struct ViewerAPI *api);

struct ViewerPickerAPI *viewer_create_picker_api(struct ViewerAPI *api);

/**
 * picker api must be dropped before any scene related modifications, or deadlock will occur
 */
void viewer_drop_picker_api(struct ViewerPickerAPI *api);

void picker_pick_nearest(struct ViewerPickerAPI *api, float x, float y);
