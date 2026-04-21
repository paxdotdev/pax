//
//  Main.swift
//  pax-app-macos
//
//  Created by Zack Brown on 10/3/23.
//

import AppKit
import SwiftUI

final class PaxMacosAppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        _ = notification
        bringApplicationToFront()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        _ = sender
        _ = flag
        bringApplicationToFront()
        return true
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        _ = sender
        return true
    }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        _ = sender
        fflush(stdout)
        fflush(stderr)
        _exit(0)
    }

    private func bringApplicationToFront() {
        let app = NSApplication.shared
        _ = app.setActivationPolicy(.regular)

        for window in app.windows where window.canBecomeKey {
            if window.isMiniaturized {
                window.deminiaturize(nil)
            }
            window.makeKeyAndOrderFront(nil)
            window.orderFrontRegardless()
        }

        if #available(macOS 14.0, *) {
            app.activate()
        } else {
            app.activate(ignoringOtherApps: true)
        }
    }
}

@main
struct pax_app_macosApp: App {
    @NSApplicationDelegateAdaptor(PaxMacosAppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup {
            PaxViewMacos()
        }
    }
}
