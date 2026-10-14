#include <gtk/gtk.h>
#include <epoxy/gl.h>
#include <math.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "gpu-import.h"

/* Native children remain live GTK widgets. Their GSK snapshot receives the same
 * allocation transform used by GTK picking; no widget screenshot is substituted. */
typedef struct {
    GtkWidget parent;
    GtkWidget *child;
    GtkWidget *scene_input;
    GdkTexture *tiles[3];
    graphene_point_t hole;
    double angle;
    gboolean masked;
} PaxProbeHost;
typedef GtkWidgetClass PaxProbeHostClass;
G_DEFINE_TYPE(PaxProbeHost, pax_probe_host, GTK_TYPE_WIDGET)
typedef struct { GtkWidget parent; PaxProbeHost *host; } PaxSceneInput;
typedef GtkWidgetClass PaxSceneInputClass;
G_DEFINE_TYPE(PaxSceneInput, pax_scene_input, GTK_TYPE_WIDGET)

static GtkWidget *window, *entry, *selectable, *nested, *outer;
static PaxProbeHost *animated;
static unsigned frames, failures;
static int selection_start, selection_end;
static gboolean test_mode, external_input;
static unsigned input_changes;
static unsigned scene_pointer_presses, entry_pointer_presses, input_wait_ticks;
static gboolean scene_click_preceded_entry;
static double initial_inner_scroll, initial_outer_scroll;
static void check(gboolean condition, const char *label);
static gboolean host_contains(GtkWidget *widget, double x, double y);

static gboolean scene_input_contains(GtkWidget *widget, double x, double y) {
    PaxSceneInput *self = (PaxSceneInput *)widget;
    /* GTK recursively picks children before the parent's contains() vfunc.
     * An input-only scene layer therefore owns points removed from native
     * coverage, while native children keep their normal GTK event path. */
    return !host_contains(GTK_WIDGET(self->host), x, y);
}
static void scene_input_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
    (void)widget; (void)snapshot;
}
static void pax_scene_input_class_init(PaxSceneInputClass *klass) {
    klass->contains = scene_input_contains;
    klass->snapshot = scene_input_snapshot;
    gtk_widget_class_set_css_name(klass, "pax-scene-input");
}
static void pax_scene_input_init(PaxSceneInput *self) { (void)self; }

static gboolean verify_import_pixels(GdkTexture *texture, double red, double green, double blue) {
    /* Diagnostic readback occurs once per test texture, never in the frame loop. */
    GdkTextureDownloader *downloader = gdk_texture_downloader_new(texture);
    gdk_texture_downloader_set_format(downloader, GDK_MEMORY_R8G8B8A8);
    gsize stride, length;
    GBytes *bytes = gdk_texture_downloader_download_bytes(downloader, &stride);
    const guint8 *pixels = g_bytes_get_data(bytes, &length);
    int expected[] = { (int)lround(red * 255), (int)lround(green * 255), (int)lround(blue * 255), 255 };
    int width = gdk_texture_get_width(texture), height = gdk_texture_get_height(texture);
    gboolean valid = length >= stride * (height - 1) + (gsize)width * 4;
    for (int y = 0; valid && y < height; y++) for (int x = 0; valid && x < width; x++)
        for (int channel = 0; channel < 4; channel++) {
            int actual = pixels[y * stride + x * 4 + channel];
            gboolean white = x >= width / 4 && x < width * 3 / 4 && y >= height / 4 && y < height * 3 / 4;
            int wanted = white ? 255 : expected[channel];
            if (abs(actual - wanted) > 2) {
                printf("PIXEL_MISMATCH x=%d y=%d channel=%d expected=%d actual=%d\n", x, y, channel, wanted, actual);
                valid = FALSE; break;
            }
        }
    g_bytes_unref(bytes);
    gdk_texture_downloader_free(downloader);
    return valid;
}

static void verify_import_lifetimes(void) {
    guint created_before = pax_probe_import_created(), released_before = pax_probe_import_released();
    gsize alive_before = pax_probe_import_live_allocations();
    gboolean pixels_valid = TRUE;
    for (int i = 0; i < 12; i++) {
        double red = (i + 1) / 16.0;
        GdkTexture *texture = pax_probe_import_gpu_tile(gdk_display_get_default(), 61 + i * 7, 37 + i * 3, red, 0.25, 0.625);
        if (!texture) { pixels_valid = FALSE; break; }
        pixels_valid &= verify_import_pixels(texture, red, 0.25, 0.625);
        g_object_unref(texture);
    }
    check(pixels_valid, "twelve varying-size imports preserve every clipped Pax pixel");
    guint created = pax_probe_import_created() - created_before;
    guint released = pax_probe_import_released() - released_before;
    gsize alive = pax_probe_import_live_allocations();
    printf("IMPORT_LIFETIMES created=%u released=%u allocations_before=%zu allocations_after=%zu\n", created, released, alive_before, alive);
    check(created == 12 && released == created, "GDK destroy callbacks release every transient imported tile");
    check(alive == alive_before, "Vulkan image and memory allocations return to baseline");
}

static void entry_changed(GtkEditable *editable, gpointer ignored) {
    (void)editable; (void)ignored;
    input_changes++;
}

static gboolean observe_pointer_press(GtkEventControllerLegacy *controller, GdkEvent *event, gpointer ignored) {
    (void)ignored;
    if (gdk_event_get_event_type(event) == GDK_BUTTON_PRESS) {
        GtkWidget *target = gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(controller));
        if (target == entry) {
            entry_pointer_presses++;
            scene_click_preceded_entry = scene_pointer_presses == 1;
            printf("NATIVE_ENTRY_POINTER_PRESS count=%u\n", entry_pointer_presses);
        } else {
            scene_pointer_presses++;
            printf("PAX_SCENE_POINTER_PRESS count=%u native_entry_count=%u\n", scene_pointer_presses, entry_pointer_presses);
        }
        fflush(stdout);
    }
    /* Observation must not consume or synthesize the control's native event. */
    return FALSE;
}

static void observe_native_pointer_events(GtkWidget *widget) {
    GtkEventController *controller = gtk_event_controller_legacy_new();
    gtk_event_controller_set_propagation_phase(controller, GTK_PHASE_CAPTURE);
    g_signal_connect(controller, "event", G_CALLBACK(observe_pointer_press), NULL);
    gtk_widget_add_controller(widget, controller);
}

static void inspect_gl_interop(GdkDisplay *display) {
    GError *error = NULL;
    GdkGLContext *context = gdk_display_create_gl_context(display, &error);
    if (context && gdk_gl_context_realize(context, &error)) {
        gdk_gl_context_make_current(context);
        printf("gl_vendor=%s gl_renderer=%s gl_version=%s\n", glGetString(GL_VENDOR), glGetString(GL_RENDERER), glGetString(GL_VERSION));
        const char *extensions[] = {"GL_EXT_memory_object", "GL_EXT_memory_object_fd", "GL_EXT_semaphore", "GL_EXT_semaphore_fd"};
        for (gsize i = 0; i < G_N_ELEMENTS(extensions); i++)
            printf("gl_extension %s=%d\n", extensions[i], epoxy_has_gl_extension(extensions[i]));
        gdk_gl_context_clear_current();
    } else {
        printf("gl_interop_unavailable=%s\n", error ? error->message : "no GL context");
    }
    g_clear_error(&error);
    g_clear_object(&context);
}

static void check(gboolean condition, const char *label) {
    printf("%s %s\n", condition ? "PASS" : "FAIL", label);
    fflush(stdout);
    if (!condition) failures++;
}

static void host_dispose(GObject *object) {
    PaxProbeHost *self = (PaxProbeHost *)object;
    g_clear_pointer(&self->scene_input, gtk_widget_unparent);
    g_clear_pointer(&self->child, gtk_widget_unparent);
    for (gsize i = 0; i < G_N_ELEMENTS(self->tiles); i++) g_clear_object(&self->tiles[i]);
    G_OBJECT_CLASS(pax_probe_host_parent_class)->dispose(object);
}

static void host_measure(GtkWidget *widget, GtkOrientation orientation, int for_size,
                         int *minimum, int *natural, int *minimum_baseline, int *natural_baseline) {
    (void)widget; (void)for_size;
    *minimum = *natural = orientation == GTK_ORIENTATION_HORIZONTAL ? 620 : 210;
    *minimum_baseline = *natural_baseline = -1;
}

static void host_allocate(GtkWidget *widget, int width, int height, int baseline) {
    PaxProbeHost *self = (PaxProbeHost *)widget;
    graphene_point_t point = GRAPHENE_POINT_INIT(24, 24);
    GskTransform *transform = gsk_transform_translate(NULL, &point);
    transform = gsk_transform_rotate(transform, (float)self->angle);
    transform = gsk_transform_scale(transform, 0.94f, 0.94f);
    /* gtk_widget_allocate takes ownership of the transform. */
    gtk_widget_allocate(self->child, MAX(1, width - 48), MAX(1, height - 48), baseline, transform);
    graphene_point_t entry_point = GRAPHENE_POINT_INIT(gtk_widget_get_width(entry) * 0.55f,
        gtk_widget_get_height(entry) / 2.0f);
    if (!gtk_widget_compute_point(entry, widget, &entry_point, &self->hole))
        self->hole = (graphene_point_t)GRAPHENE_POINT_INIT(width * 0.55f, 40);
    gtk_widget_allocate(self->scene_input, width, height, -1, NULL);
}

static GskPath *host_visible_path(PaxProbeHost *self) {
    GskPathBuilder *builder = gsk_path_builder_new();
    graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0,
        gtk_widget_get_width(GTK_WIDGET(self)), gtk_widget_get_height(GTK_WIDGET(self)));
    gsk_path_builder_add_rect(builder, &bounds);
    float x = self->hole.x, y = self->hole.y;
    /* An asymmetric curved hole exercises a general path, not an inset rect or
     * rounded-window shape. Snapshot and picking consume this same path. */
    gsk_path_builder_move_to(builder, x, y - 18);
    gsk_path_builder_cubic_to(builder, x + 28, y - 18, x + 24, y + 16, x, y + 20);
    gsk_path_builder_cubic_to(builder, x - 27, y + 12, x - 16, y - 20, x, y - 18);
    gsk_path_builder_close(builder);
    return gsk_path_builder_free_to_path(builder);
}

static void host_push_native_mask(PaxProbeHost *self, GtkSnapshot *snapshot) {
    float width = gtk_widget_get_width(GTK_WIDGET(self)), height = gtk_widget_get_height(GTK_WIDGET(self));
    GdkRGBA opaque = {1, 1, 1, 1}, translucent = {1, 1, 1, 0.30};
    GskPath *path = host_visible_path(self);
    gtk_snapshot_push_mask(snapshot, GSK_MASK_MODE_ALPHA);
    gtk_snapshot_append_fill(snapshot, path, GSK_FILL_RULE_EVEN_ODD, &opaque);
    gtk_snapshot_pop(snapshot);
    gsk_path_unref(path);
    gtk_snapshot_push_mask(snapshot, GSK_MASK_MODE_ALPHA);
    graphene_rect_t solid = GRAPHENE_RECT_INIT(0, 0, width * 0.7f, height);
    graphene_rect_t faded = GRAPHENE_RECT_INIT(width * 0.7f, 0, width * 0.3f, height);
    gtk_snapshot_append_color(snapshot, &opaque, &solid);
    gtk_snapshot_append_color(snapshot, &translucent, &faded);
    gtk_snapshot_pop(snapshot);
}

static gboolean host_contains(GtkWidget *widget, double x, double y) {
    PaxProbeHost *self = (PaxProbeHost *)widget;
    int width = gtk_widget_get_width(widget), height = gtk_widget_get_height(widget);
    graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, width, height);
    GskRoundedRect clip;
    gsk_rounded_rect_init_from_rect(&clip, &rect, 28);
    graphene_point_t point = GRAPHENE_POINT_INIT(x, y);
    if (!gsk_rounded_rect_contains_point(&clip, &point)) return FALSE;
    if (!self->masked) return TRUE;
    GskPath *path = host_visible_path(self);
    gboolean visible = gsk_path_in_fill(path, &point, GSK_FILL_RULE_EVEN_ODD);
    gsk_path_unref(path);
    /* Fractional opacity remains interactive. Only zero coverage passes input
     * through to the underlying scene; alpha is not an input probability. */
    return visible;
}

static void host_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
    PaxProbeHost *self = (PaxProbeHost *)widget;
    float width = gtk_widget_get_width(widget), height = gtk_widget_get_height(widget);
    graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, width, height);
    GskRoundedRect clip;
    gsk_rounded_rect_init_from_rect(&clip, &rect, 28);
    gtk_snapshot_push_rounded_clip(snapshot, &clip);
    GdkRGBA background = {0.88, 0.93, 0.98, 1};
    gtk_snapshot_append_color(snapshot, &background, &rect);
    /* Imported Pax-rendered tiles share this GSK composition with live GTK
     * controls. The partial native mask reveals their pixels underneath. */
    for (gsize i = 0; i < G_N_ELEMENTS(self->tiles); i++) if (self->tiles[i]) {
        graphene_rect_t tile = GRAPHENE_RECT_INIT(width * i / 3, 0, width / 3, height);
        gtk_snapshot_push_opacity(snapshot, 0.28);
        gtk_snapshot_append_texture(snapshot, self->tiles[i], &tile);
        gtk_snapshot_pop(snapshot);
    }
    /* The right strip partially occludes actual widgets. Snapshot masks support
     * fractional alpha, unlike native-window shape regions. */
    if (self->masked) {
        host_push_native_mask(self, snapshot);
    }
    gtk_widget_snapshot_child(widget, self->child, snapshot);
    if (self->masked) { gtk_snapshot_pop(snapshot); gtk_snapshot_pop(snapshot); }
    gtk_snapshot_pop(snapshot);
}

static void pax_probe_host_class_init(PaxProbeHostClass *klass) {
    G_OBJECT_CLASS(klass)->dispose = host_dispose;
    klass->measure = host_measure;
    klass->size_allocate = host_allocate;
    klass->snapshot = host_snapshot;
    klass->contains = host_contains;
    gtk_widget_class_set_css_name(klass, "pax-probe-host");
}
static void pax_probe_host_init(PaxProbeHost *self) { self->masked = TRUE; }

static GtkWidget *host_new(GtkWidget *child, double angle) {
    PaxProbeHost *self = g_object_new(pax_probe_host_get_type(), NULL);
    self->child = child;
    self->angle = angle;
    gtk_widget_set_parent(child, GTK_WIDGET(self));
    PaxSceneInput *scene_input = g_object_new(pax_scene_input_get_type(), NULL);
    scene_input->host = self;
    self->scene_input = GTK_WIDGET(scene_input);
    gtk_widget_set_parent(self->scene_input, GTK_WIDGET(self));
    gtk_widget_set_overflow(GTK_WIDGET(self), GTK_OVERFLOW_HIDDEN);
    return GTK_WIDGET(self);
}

static void save_capture(void) {
    const char *path = g_getenv("PAX_PROBE_SCREENSHOT");
    if (!path) return;
    GdkPaintable *paintable = gtk_widget_paintable_new(window);
    GtkSnapshot *snapshot = gtk_snapshot_new();
    int width = gtk_widget_get_width(window), height = gtk_widget_get_height(window);
    gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot), width, height);
    GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
    if (node) {
        GskRenderer *renderer = gtk_native_get_renderer(GTK_NATIVE(window));
        graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0, width, height);
        GdkTexture *texture = gsk_renderer_render_texture(renderer, node, &bounds);
        check(texture && gdk_texture_save_to_png(texture, path), "one-shot visual QA capture");
        g_clear_object(&texture);
        gsk_render_node_unref(node);
    } else check(FALSE, "window snapshot has render nodes");
    g_object_unref(paintable);
}

static gboolean belongs_to(GtkWidget *widget, GtkWidget *ancestor) {
    return widget == ancestor || (widget && gtk_widget_is_ancestor(widget, ancestor));
}

static void verify_mask_picking(void) {
    GtkWidget *host = GTK_WIDGET(animated);
    graphene_point_t fraction_entry = GRAPHENE_POINT_INIT(gtk_widget_get_width(entry) * 0.85f,
        gtk_widget_get_height(entry) / 2.0f), fraction, mapped;
    gboolean fraction_ok = gtk_widget_compute_point(entry, host, &fraction_entry, &fraction);
    gboolean hole_ok = gtk_widget_compute_point(host, window, &animated->hole, &mapped);
    GtkWidget *picked = hole_ok ? gtk_widget_pick(window, mapped.x, mapped.y, GTK_PICK_DEFAULT) : NULL;
    check(hole_ok && !gtk_widget_contains(host, animated->hole.x, animated->hole.y)
        && picked == animated->scene_input,
        "transparent curved hole gives pointer ownership to the Pax scene input layer");
    gboolean mapped_ok = fraction_ok && gtk_widget_compute_point(host, window, &fraction, &mapped);
    picked = mapped_ok ? gtk_widget_pick(window, mapped.x, mapped.y, GTK_PICK_DEFAULT) : NULL;
    check(mapped_ok && fraction.x > gtk_widget_get_width(host) * 0.7f
        && gtk_widget_contains(host, fraction.x, fraction.y) && belongs_to(picked, entry),
        "fractional-alpha native entry remains interactive after transform and scroll");
    check(!gtk_widget_contains(host, 1, 1) && !gtk_widget_contains(host, gtk_widget_get_width(host) + 1, 40),
        "rounded and hard clip edges reject GTK picking");

    /* Sample the actual shared GSK mask stack once, independently of control
     * theme colors, and compare its coverage with the host's picking contract. */
    int width = gtk_widget_get_width(host), height = gtk_widget_get_height(host);
    graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0, width, height);
    GskRoundedRect clip;
    gsk_rounded_rect_init_from_rect(&clip, &bounds, 28);
    GtkSnapshot *snapshot = gtk_snapshot_new();
    gtk_snapshot_push_rounded_clip(snapshot, &clip);
    host_push_native_mask(animated, snapshot);
    GdkRGBA white = {1, 1, 1, 1};
    gtk_snapshot_append_color(snapshot, &white, &bounds);
    gtk_snapshot_pop(snapshot); gtk_snapshot_pop(snapshot); gtk_snapshot_pop(snapshot);
    GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
    GdkTexture *texture = gsk_renderer_render_texture(gtk_native_get_renderer(GTK_NATIVE(window)), node, &bounds);
    GdkTextureDownloader *downloader = gdk_texture_downloader_new(texture);
    gdk_texture_downloader_set_format(downloader, GDK_MEMORY_R8G8B8A8);
    gsize stride;
    GBytes *bytes = gdk_texture_downloader_download_bytes(downloader, &stride);
    const guint8 *pixels = g_bytes_get_data(bytes, NULL);
    int hole_alpha = pixels[(int)animated->hole.y * stride + (int)animated->hole.x * 4 + 3];
    int fraction_alpha = fraction_ok ? pixels[(int)fraction.y * stride + (int)fraction.x * 4 + 3] : -1;
    int solid_alpha = pixels[(height / 2) * stride + (width / 4) * 4 + 3];
    int corner_alpha = pixels[stride + 4 + 3];
    printf("MASK_COVERAGE hole=%d fractional=%d solid=%d rounded_corner=%d\n", hole_alpha, fraction_alpha, solid_alpha, corner_alpha);
    check(hole_alpha == 0 && abs(fraction_alpha - 77) <= 1 && solid_alpha == 255 && corner_alpha == 0,
        "GSK path and alpha mask pixels agree with native input coverage");
    g_bytes_unref(bytes);
    gdk_texture_downloader_free(downloader);
    g_object_unref(texture);
    gsk_render_node_unref(node);
}

static gboolean finish_test(gpointer ignored) {
    (void)ignored;
    check(!strcmp(gtk_editable_get_text(GTK_EDITABLE(entry)), "Pax native edit retained"), "editable text survives transformed snapshots");
    int start, end;
    if (external_input) {
        check(input_changes > 0, "native GTK changed signal received external keyboard input");
        check(gtk_editable_get_selection_bounds(GTK_EDITABLE(entry), &start, &end)
            && start == 0 && end == g_utf8_strlen(gtk_editable_get_text(GTK_EDITABLE(entry)), -1),
            "real Ctrl+A selection survives native animation and nested scrolling");
        check(scene_pointer_presses == 1 && entry_pointer_presses == 1 && scene_click_preceded_entry,
            "real hole click belongs to Pax scene and fractional click belongs to native Entry");
    } else check(gtk_editable_get_selection_bounds(GTK_EDITABLE(entry), &start, &end)
        && start == selection_start && end == selection_end, "selection survives animation");
    GtkWidget *focus = gtk_root_get_focus(GTK_ROOT(window));
    check(focus == entry || (focus && gtk_widget_is_ancestor(focus, entry)), "native keyboard focus survives animation");
    graphene_point_t point = GRAPHENE_POINT_INIT(30, gtk_widget_get_height(entry)/2.0f), mapped;
    gboolean mapped_ok = gtk_widget_compute_point(entry, window, &point, &mapped);
    GtkWidget *picked = mapped_ok ? gtk_widget_pick(window, mapped.x, mapped.y, GTK_PICK_DEFAULT) : NULL;
    check(picked == entry || (picked && gtk_widget_is_ancestor(picked, entry)), "GTK picking follows native allocation transform");
    check(gtk_label_get_selectable(GTK_LABEL(selectable)), "text remains a selectable native label");
    double inner_scroll = gtk_adjustment_get_value(gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(nested)));
    double outer_scroll = gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(outer)));
    check(inner_scroll > initial_inner_scroll && fabs(inner_scroll - 160) < 1,
        "nested native horizontal scroller retains its changed offset");
    check(outer_scroll > initial_outer_scroll && fabs(outer_scroll - 24) < 1,
        "outer native vertical scroller changes independently");
    verify_mask_picking();
    if (g_getenv("PAX_PROBE_GPU_LIBRARY") && pax_probe_import_created() > 0) verify_import_lifetimes();
    save_capture();
    printf("RESULT failures=%u frames=%u backend=%s renderer=%s\n", failures, frames,
        G_OBJECT_TYPE_NAME(gdk_display_get_default()), G_OBJECT_TYPE_NAME(gtk_native_get_renderer(GTK_NATIVE(window))));
    gtk_window_destroy(GTK_WINDOW(window));
    return G_SOURCE_REMOVE;
}

static gboolean animate(GtkWidget *widget, GdkFrameClock *clock, gpointer ignored) {
    (void)widget; (void)clock; (void)ignored;
    frames++;
    animated->angle = -2.0 + sin(frames * 0.06) * 1.5;
    gtk_widget_queue_allocate(GTK_WIDGET(animated));
    if (frames == 20) gtk_adjustment_set_value(gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(nested)), 160);
    if (frames == 40) gtk_adjustment_set_value(gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(outer)), 24);
    if (test_mode && frames == (external_input ? 180u : 60u)) { g_idle_add(finish_test, NULL); return G_SOURCE_REMOVE; }
    return G_SOURCE_CONTINUE;
}

static gboolean begin_after_external_input(gpointer ignored) {
    (void)ignored;
    int start, end;
    gboolean input_complete = scene_pointer_presses == 1 && entry_pointer_presses == 1
        && !strcmp(gtk_editable_get_text(GTK_EDITABLE(entry)), "Pax native edit retained")
        && gtk_editable_get_selection_bounds(GTK_EDITABLE(entry), &start, &end)
        && start == 0 && end == g_utf8_strlen(gtk_editable_get_text(GTK_EDITABLE(entry)), -1);
    if (!input_complete && ++input_wait_ticks < 250) return G_SOURCE_CONTINUE;
    check(input_complete, "external mouse and keyboard sequence completed before animation");
    gtk_widget_add_tick_callback(window, animate, NULL, NULL);
    return G_SOURCE_REMOVE;
}

static gboolean start_test(gpointer ignored) {
    (void)ignored;
    gtk_widget_grab_focus(entry);
    initial_inner_scroll = gtk_adjustment_get_value(gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(nested)));
    initial_outer_scroll = gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(outer)));
    if (!external_input) {
        gtk_editable_set_text(GTK_EDITABLE(entry), "Pax native edit retained");
        gtk_editable_select_region(GTK_EDITABLE(entry), 4, 10);
        gtk_editable_get_selection_bounds(GTK_EDITABLE(entry), &selection_start, &selection_end);
    } else {
        graphene_point_t hole, fraction, entry_point = GRAPHENE_POINT_INIT(gtk_widget_get_width(entry) * 0.85f,
            gtk_widget_get_height(entry) / 2.0f);
        gboolean points_valid = gtk_widget_compute_point(GTK_WIDGET(animated), window, &animated->hole, &hole)
            && gtk_widget_compute_point(entry, window, &entry_point, &fraction);
        check(points_valid, "GTK computes actual window coordinates for external mouse input");
        if (points_valid) {
            double surface_x, surface_y;
            gtk_native_get_surface_transform(GTK_NATIVE(window), &surface_x, &surface_y);
            printf("INPUT_POINTS %d %d %d %d\n", (int)lround(hole.x - surface_x), (int)lround(hole.y - surface_y),
                (int)lround(fraction.x - surface_x), (int)lround(fraction.y - surface_y));
        }
    }
    printf("INPUT_READY\n"); fflush(stdout);
    if (external_input) g_timeout_add(20, begin_after_external_input, NULL);
    else gtk_widget_add_tick_callback(window, animate, NULL, NULL);
    return G_SOURCE_REMOVE;
}

static void activate(GtkApplication *app, gpointer ignored) {
    (void)ignored;
    window = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(window), "PAX-1038 GTK native composition probe");
    gtk_window_set_default_size(GTK_WINDOW(window), 900, 720);
    outer = gtk_scrolled_window_new();
    GtkWidget *content = gtk_box_new(GTK_ORIENTATION_VERTICAL, 22);
    gtk_widget_set_margin_start(content, 40); gtk_widget_set_margin_end(content, 40);
    gtk_widget_set_margin_top(content, 24); gtk_widget_set_margin_bottom(content, 24);
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(outer), content);
    gtk_window_set_child(GTK_WINDOW(window), outer);
    gtk_box_append(GTK_BOX(content), gtk_label_new("Real GTK widgets · transformed controls · curved input hole · fractional alpha mask"));
    GtkWidget *controls = gtk_box_new(GTK_ORIENTATION_VERTICAL, 10);
    entry = gtk_entry_new();
    gtk_editable_set_text(GTK_EDITABLE(entry), "Type here: native GTK editing and IME");
    g_signal_connect(entry, "changed", G_CALLBACK(entry_changed), NULL);
    gtk_box_append(GTK_BOX(controls), entry);
    selectable = gtk_label_new("Selectable native text remains selectable through a transformed scene host.");
    gtk_label_set_selectable(GTK_LABEL(selectable), TRUE);
    gtk_label_set_wrap(GTK_LABEL(selectable), TRUE);
    gtk_box_append(GTK_BOX(controls), selectable);
    GtkWidget *buttons = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 12);
    gtk_box_append(GTK_BOX(buttons), gtk_button_new_with_label("Native button"));
    gtk_box_append(GTK_BOX(buttons), gtk_check_button_new_with_label("Native checkbox"));
    gtk_box_append(GTK_BOX(controls), buttons);
    animated = (PaxProbeHost *)host_new(controls, -2);
    observe_native_pointer_events(entry);
    observe_native_pointer_events(animated->scene_input);
    gtk_box_append(GTK_BOX(content), GTK_WIDGET(animated));
    nested = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(GTK_SCROLLED_WINDOW(nested), GTK_POLICY_ALWAYS, GTK_POLICY_NEVER);
    gtk_widget_set_size_request(nested, -1, 190);
    GtkWidget *row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 14);
    for (int i = 0; i < 8; i++) {
        char title[80]; snprintf(title, sizeof(title), "Native nested scroll tile %d", i);
        GtkWidget *tile = gtk_button_new_with_label(title);
        gtk_widget_set_size_request(tile, 230, 130);
        gtk_box_append(GTK_BOX(row), tile);
    }
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(nested), row);
    gtk_box_append(GTK_BOX(content), nested);
    if (g_getenv("PAX_PROBE_GPU_LIBRARY")) {
        GtkWidget *gpu_row = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 10);
        GdkDisplay *display = gtk_widget_get_display(window);
        for (int i = 0; i < 3; i++) {
            GdkTexture *texture = pax_probe_import_gpu_tile(display, 160, 90,
                i == 0 ? 0.8 : 0.05, i == 1 ? 0.8 : 0.1, i == 2 ? 0.9 : 0.15);
            check(texture != NULL, "WGPU28 native texture import");
            if (texture) {
                check(verify_import_pixels(texture, i == 0 ? 0.8 : 0.05, i == 1 ? 0.8 : 0.1, i == 2 ? 0.9 : 0.15),
                    "imported tile pixels equal clipped Pax vector scene");
                animated->tiles[i] = g_object_ref(texture);
                GtkWidget *picture = gtk_picture_new_for_paintable(GDK_PAINTABLE(texture));
                gtk_widget_set_size_request(picture, 160, 90);
                gtk_box_append(GTK_BOX(gpu_row), picture);
                g_object_unref(texture);
            }
        }
        gtk_box_append(GTK_BOX(content), gpu_row);
    }
    for (int i = 0; i < 8; i++) {
        char title[100]; snprintf(title, sizeof(title), "Outer scroller row %d — real GTK controls, not GPU tile imports", i);
        GtkWidget *label = gtk_label_new(title); gtk_widget_set_size_request(label, -1, 90);
        gtk_box_append(GTK_BOX(content), label);
    }
    gtk_window_present(GTK_WINDOW(window));
    GdkDisplay *display = gtk_widget_get_display(window);
    GdkDmabufFormats *formats = gdk_display_get_dmabuf_formats(display);
    printf("gtk=%u.%u.%u display=%s dmabuf_formats=%zu\n", gtk_get_major_version(), gtk_get_minor_version(),
        gtk_get_micro_version(), G_OBJECT_TYPE_NAME(display), gdk_dmabuf_formats_get_n_formats(formats));
    for (gsize i = 0; i < MIN((gsize)12, gdk_dmabuf_formats_get_n_formats(formats)); i++) {
        guint32 fourcc; guint64 modifier;
        gdk_dmabuf_formats_get_format(formats, i, &fourcc, &modifier);
        printf("dmabuf fourcc=%08x modifier=%016" G_GINT64_MODIFIER "x\n", fourcc, modifier);
    }
    inspect_gl_interop(display);
    fflush(stdout);
    if (test_mode) g_timeout_add(500, start_test, NULL);
}

int main(int argc, char **argv) {
    test_mode = g_getenv("PAX_PROBE_SELF_TEST") != NULL;
    external_input = g_getenv("PAX_PROBE_EXTERNAL_INPUT") != NULL;
    GtkApplication *app = gtk_application_new("dev.pax.NativeCompositionProbe", G_APPLICATION_NON_UNIQUE);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int result = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    pax_probe_gpu_shutdown();
    return failures ? 1 : result;
}
