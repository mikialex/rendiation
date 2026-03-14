#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct ViewerContentAPIInstance ViewerContentAPIInstance;

struct ViewerContentAPIInstance *create_viewer_content_api_instance(void);

void drop_viewer_content_api_instance(struct ViewerContentAPIInstance *instance);
