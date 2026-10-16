#pragma once
#include <gtk/gtk.h>
GdkTexture *pax_probe_import_gpu_tile(GdkDisplay *display, guint width, guint height, double red, double green, double blue);
void pax_probe_gpu_shutdown(void);
guint pax_probe_import_created(void);
guint pax_probe_import_released(void);
guint pax_probe_pending_producer_imports(void);
int pax_probe_delayed_producer_ready(void);
gsize pax_probe_import_live_allocations(void);
