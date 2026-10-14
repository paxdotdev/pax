#pragma once
#include <gtk/gtk.h>
GdkTexture *pax_probe_import_gpu_tile(GdkDisplay *display, guint width, guint height, double red, double green, double blue);
void pax_probe_gpu_shutdown(void);
guint pax_probe_import_created(void);
guint pax_probe_import_released(void);
gsize pax_probe_import_live_allocations(void);
