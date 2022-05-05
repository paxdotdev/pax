#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Affine {
  double coefficients[6];
} Affine;

typedef enum TextSize_Tag {
  Auto,
  Pixels,
} TextSize_Tag;

typedef struct Auto_Body {

} Auto_Body;

typedef struct TextSize {
  TextSize_Tag tag;
  union {
    Auto_Body auto_;
    struct {
      double pixels;
    };
  };
} TextSize;

typedef struct TextPatchMessage {
  const char *content;
  const struct Affine *transform;
  const struct TextSize *size_x;
  const struct TextSize *size_y;
} TextPatchMessage;

typedef struct ClippingPatch {
  const struct TextSize *size_x;
  const struct TextSize *size_y;
  const struct Affine *transform;
} ClippingPatch;

typedef struct NativeArgsClick {
  double x;
  double y;
} NativeArgsClick;

typedef enum NativeMessage_Tag {
  TextCreate,
  TextUpdate,
  TextDelete,
  ClippingCreate,
  ClippingUpdate,
  ClippingDelete,
  NativeEventClick,
} NativeMessage_Tag;

typedef struct TextUpdate_Body {
  uint64_t _0;
  struct TextPatchMessage _1;
} TextUpdate_Body;

typedef struct ClippingUpdate_Body {
  uint64_t _0;
  struct ClippingPatch _1;
} ClippingUpdate_Body;

typedef struct NativeMessage {
  NativeMessage_Tag tag;
  union {
    struct {
      uint64_t text_create;
    };
    TextUpdate_Body text_update;
    struct {
      uint64_t text_delete;
    };
    struct {
      uint64_t clipping_create;
    };
    ClippingUpdate_Body clipping_update;
    struct {
      uint64_t clipping_delete;
    };
    struct {
      struct NativeArgsClick native_event_click;
    };
  };
} NativeMessage;

typedef struct TextCommand {
  const char *set_font;
  const char *set_weight;
  const char *set_fill_color;
  const char *set_stroke_color;
  const char *set_decoration;
} TextCommand;

typedef struct NativeMessageQueue {
  const struct NativeMessage *msg_ptr;
  uint64_t length;
} NativeMessageQueue;


typedef struct PaxEngineContainer PaxEngineContainer;
typedef struct PaxMessageQueueContainer PaxMessageQueueContainer;


struct PaxEngineContainer *pax_init(void (*logger)(const char*));

struct NativeMessageQueue *pax_tick(struct PaxEngineContainer *bridge_container,
                                          void *cgContext,
                                          float width,
                                          float height);

void pax_cleanup_message_queue(struct NativeMessage *queue);
