//
//  Main.swift
//  pax-app-macos
//
//  Created by Zack Brown on 10/3/23.
//

import SwiftUI

final class PaxMacosAppDelegate: NSObject, NSApplicationDelegate {
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
