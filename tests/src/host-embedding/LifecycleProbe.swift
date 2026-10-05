// Automated ABI/lifecycle regression. No input synthesis or screenshot capture:
// the caller drives its own view and uses the same exported APIs as an embedder.
import Foundation
#if os(macOS)
import AppKit
typealias ProbeView = NSView
#else
import UIKit
typealias ProbeView = UIView
#endif

final class LifecycleProbe {
    let region = ProbeView(frame: .zero)
    weak var controller: AnyObject?
    private var handle: UnsafeMutableRawPointer?
    private var generation = 0
    private var expanded = false
    private let deadline = Date().addingTimeInterval(15)

    func start() {
        // This initializer creates its executor before decoding the invalid data.
        let invalid = Array("invalid counter".utf8)
        let failed = invalid.withUnsafeBufferPointer { pax_host_prepare_with_data($0.baseAddress, $0.count) }
        require(failed == nil, "failed initializer returned a session")
        print("PASS failed initialization reported")
        DispatchQueue.main.asyncAfter(deadline: .now() + 15) { self.fail("lifecycle acknowledgement timed out") }
        prepareWhenCleanupAcknowledges()
    }

    private func prepareWhenCleanupAcknowledges() {
        let bytes = Array("7".utf8)
        guard let prepared = bytes.withUnsafeBufferPointer({ pax_host_prepare_with_data($0.baseAddress, $0.count) }) else {
            require(Date() < deadline, "failed initialization retained its mount reservation")
            // Test-only bounded retry observes the host's independent cleanup;
            // it never pumps the engine or drives a production shutdown path.
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) { self.prepareWhenCleanupAcknowledges() }
            return
        }
        generation += 1
        expanded = false
        region.frame = CGRect(x: 0, y: 0, width: 640, height: 0)
        handle = prepared
        pax_host_observe(prepared, Unmanaged.passUnretained(self).toOpaque(), { context, phase in
            Unmanaged<LifecycleProbe>.fromOpaque(context!).takeUnretainedValue().phase(phase)
        }, { _ in })
        let owner = controller.map { Unmanaged.passUnretained($0).toOpaque() }
        require(pax_host_mount(prepared, Unmanaged.passUnretained(region).toOpaque(), owner) == 0, "mount failed")
        require(pax_host_prepare() == nil, "duplicate prepare was accepted")
    }

    private func phase(_ phase: UInt32) {
        if phase == 2 {
            require(region.bounds.height == 0, "initial region was not collapsed")
            print("PASS generation \(generation) ready while collapsed")
            request()
        } else if phase == 4 {
            guard let handle else { fail("duplicate closed notification") }
            self.handle = nil
            pax_host_release(handle)
            print("PASS generation \(generation) cleanup acknowledged")
            if generation == 1 { prepareWhenCleanupAcknowledges() }
            else { print("PASS native host lifecycle probe"); exit(0) }
        }
    }

    private func request() {
        guard let handle else { fail("request without session") }
        let bytes = Array("increment".utf8)
        bytes.withUnsafeBufferPointer {
            pax_host_request(handle, $0.baseAddress, $0.count, Unmanaged.passUnretained(self).toOpaque()) { context, status, bytes, count in
                let probe = Unmanaged<LifecycleProbe>.fromOpaque(context!).takeUnretainedValue()
                let copied = bytes.map { Array(UnsafeBufferPointer(start: $0, count: count)) } ?? []
                probe.reply(status, String(decoding: copied, as: UTF8.self))
            }
        }
    }

    private func reply(_ status: UInt32, _ text: String) {
        require(status == 0 && text == (expanded ? "9" : "8"), "unexpected reply \(status): \(text)")
        print("PASS generation \(generation) request replied \(text)")
        if generation == 1 && !expanded {
            expanded = true
            region.frame.size.height = 400
            #if os(macOS)
            region.layoutSubtreeIfNeeded()
            #else
            region.layoutIfNeeded()
            #endif
            request()
        } else {
            // Close both an expanded mount and a replacement that stayed at zero.
            pax_host_close(handle!)
        }
    }
    private func require(_ condition: Bool, _ message: String) {
        if !condition { fail(message) }
    }
    private func fail(_ message: String) -> Never {
        print("FAIL \(message)")
        exit(1)
    }
}

#if os(macOS)
@main
final class ProbeApplication: NSObject, NSApplicationDelegate {
    let probe = LifecycleProbe()
    var window: NSWindow!
    static func main() {
        let delegate = ProbeApplication()
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        window = NSWindow(contentRect: CGRect(x: 0, y: 0, width: 640, height: 480), styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView!.addSubview(probe.region)
        probe.start()
    }
}
#else
final class ProbeController: UIViewController {
    let probe = LifecycleProbe()
    override func viewDidLoad() {
        super.viewDidLoad()
        view.addSubview(probe.region)
        probe.controller = self
        probe.start()
    }
}
@main
final class ProbeApplication: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    static func main() { UIApplicationMain(CommandLine.argc, CommandLine.unsafeArgv, nil, NSStringFromClass(ProbeApplication.self)) }
    func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = ProbeController()
        self.window = window
        window.makeKeyAndVisible()
        return true
    }
}
#endif
