//
//  Main.swift
//  pax-app-macos
//
//  Created by Zack Brown on 10/3/23.
//

import AppKit
import SwiftUI

final class PaxMacosAppDelegate: NSObject, NSApplicationDelegate {
    private static var fallbackWindow: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        _ = notification
        bringApplicationToFront()
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.0) {
            self.ensureInitialWindowIfNeeded()
            self.bringApplicationToFront()
        }
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        _ = sender
        if !flag {
            ensureInitialWindowIfNeeded()
        }
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

    private func ensureInitialWindowIfNeeded() {
        let app = NSApplication.shared
        guard !app.windows.contains(where: { $0.canBecomeKey && $0.isVisible && !$0.isMiniaturized }) else {
            return
        }

        let title = (Bundle.main.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String)
            ?? (Bundle.main.object(forInfoDictionaryKey: "CFBundleName") as? String)
            ?? "Pax"
        let hostingController = NSHostingController(rootView: PaxViewMacos())
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 900, height: 450),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = title
        window.isReleasedWhenClosed = false
        window.contentViewController = hostingController
        window.center()
        window.makeKeyAndOrderFront(nil)
        Self.fallbackWindow = window
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
