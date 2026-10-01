// Inject only into the local macOS fixture to reproduce unavailable displays
// without locking the workstation. Removing the marker permits real CoreVideo
// startup on the next application/display wake notification.
#include <CoreVideo/CoreVideo.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <signal.h>
#import <AppKit/AppKit.h>

// A deterministic host-notification trigger avoids changing the workstation's
// real lock/sleep state. The signal exists only in this injected test process.
static dispatch_source_t wake_signal;
__attribute__((constructor)) static void install_wake_probe(void) {
    signal(SIGUSR1, SIG_IGN);
    dispatch_async(dispatch_get_main_queue(), ^{
        wake_signal = dispatch_source_create(DISPATCH_SOURCE_TYPE_SIGNAL, SIGUSR1, 0, dispatch_get_main_queue());
        dispatch_source_set_event_handler(wake_signal, ^{
            fprintf(stderr, "display-link probe: delivering screens-did-wake\n");
            [NSWorkspace.sharedWorkspace.notificationCenter
                postNotificationName:NSWorkspaceScreensDidWakeNotification object:nil];
        });
        dispatch_resume(wake_signal);
    });
}

static int blocked(const char *stage) {
    const char *marker = getenv("PAX_DISPLAY_LINK_PROBE_BLOCK");
    const char *requested = getenv("PAX_DISPLAY_LINK_PROBE_STAGE");
    return marker && requested && strcmp(requested, stage) == 0 && access(marker, F_OK) == 0;
}

static CVReturn probe_create(CVDisplayLinkRef *link) {
    if (blocked("create")) {
        *link = NULL;
        fprintf(stderr, "display-link probe: creation rejected\n");
        return kCVReturnError;
    }
    CVReturn result = CVDisplayLinkCreateWithActiveCGDisplays(link);
    fprintf(stderr, "display-link probe: creation returned %d\n", result);
    return result;
}

static CVReturn probe_start(CVDisplayLinkRef link) {
    if (blocked("start")) {
        fprintf(stderr, "display-link probe: start rejected\n");
        return kCVReturnError;
    }
    CVReturn result = CVDisplayLinkStart(link);
    fprintf(stderr, "display-link probe: start returned %d\n", result);
    return result;
}

__attribute__((used)) static struct {
    const void *replacement;
    const void *original;
} interpositions[] __attribute__((section("__DATA,__interpose"))) = {
    {(const void *)probe_create, (const void *)CVDisplayLinkCreateWithActiveCGDisplays},
    {(const void *)probe_start, (const void *)CVDisplayLinkStart},
};
