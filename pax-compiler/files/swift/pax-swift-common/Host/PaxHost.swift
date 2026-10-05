import Foundation
import SwiftUI
import Rendering
import Messages
#if os(macOS)
import AppKit
typealias PaxPlatformView = NSView
typealias PaxPlatformController = NSHostingController<PaxViewMacos>
#else
import UIKit
import PaxCartridge
typealias PaxPlatformView = UIView
typealias PaxPlatformController = UIHostingController<PaxViewIos>
#endif

// This module is the host side of a versioned C boundary. It never receives a
// Rust root, Property, runtime handle or closure from the application cartridge.
final class PaxHostSession {
    // Reserve the single native-control namespace until cleanup acknowledges.
    static var active: PaxHostSession?
    static var lastError = "Pax host operation failed"
    var status: UInt32 = 0 {
        didSet {
            let snapshot = status
            // Observers may reenter host APIs. Deliver only after the transition
            // has completed, retaining the callback context through delivery.
            DispatchQueue.main.async { self.notify?(self.context, snapshot) }
        }
    }
    private var context: UnsafeMutableRawPointer?
    private var notify: (@convention(c) (UnsafeMutableRawPointer?, UInt32) -> Void)?
    private var release: (@convention(c) (UnsafeMutableRawPointer?) -> Void)?
    private var controller: PaxPlatformController?
    var stopCanvas: (() -> Void)?
    var isStandalone = false
    private var engine: OpaquePointer?
    #if os(macOS)
    var windowTitle: String?
    var windowSize = NSSize(width: 900, height: 450)
    var windowMinimum = NSSize.zero
    var windowResizable = true
    func configureWindow(_ window: NSWindow) {
        window.title = windowTitle ?? (Bundle.main.object(forInfoDictionaryKey: "CFBundleDisplayName") as? String) ?? "Pax"
        window.contentMinSize = windowMinimum
        window.setContentSize(windowSize)
        if windowResizable { window.styleMask.insert(.resizable) } else { window.styleMask.remove(.resizable) }
    }
    #endif

    static func fail(_ message: String) -> Int32 { lastError = message; return -1 }

    static func prepare(data: [UInt8] = []) -> PaxHostSession? {
        guard Thread.isMainThread else { _ = fail("Pax initialization requires the process main thread"); return nil }
        guard active == nil else { _ = fail("Pax supports one live native mount; await closed before preparing another"); return nil }
        let session = PaxHostSession()
        active = session
        #if os(macOS)
        session.engine = PaxCartridgeRuntime.shared.initEngine(width: 0, height: 0, data: data)
        PaxViewMacos.PaxEngineContainer.paxEngineContainer = session.engine
        #else
        guard pax_cartridge_abi_version() == 2 else {
            active = nil
            _ = fail("Incompatible Pax cartridge ABI")
            return nil
        }
        pax_set_shutdown_waker(1) { _ in
            DispatchQueue.main.async { PaxHostSession.cleanupWake(pending: pax_pump_shutdowns()) }
        }
        session.engine = data.withUnsafeBufferPointer { pax_init_with_data(0, 0, $0.baseAddress, Int($0.count)) }
        PaxViewIos.PaxEngineContainer.paxEngineContainer = session.engine
        #endif
        guard session.engine != nil else {
            _ = fail("Cartridge initialization failed; see the initializer diagnostic")
            session.close()
            return nil
        }
        return session
    }

    func observe(_ context: UnsafeMutableRawPointer?, _ notify: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32) -> Void,
                 _ release: @escaping @convention(c) (UnsafeMutableRawPointer?) -> Void) {
        self.release?(self.context)
        self.context = context
        self.notify = notify
        self.release = release
        notify(context, status)
    }

    func mount(_ parent: PaxPlatformView, owner: AnyObject?) -> Int32 {
        guard Thread.isMainThread, Self.active === self, status == 0 else {
            return Self.fail("Prepared Pax application must be mounted once on the main thread")
        }
        #if os(macOS)
        let child = NSHostingController(rootView: PaxViewMacos())
        #else
        guard let owner = owner as? UIViewController, parent === owner.view || parent.isDescendant(of: owner.view) else {
            return Self.fail("iOS mounting requires the container's owning UIViewController")
        }
        let child = UIHostingController(rootView: PaxViewIos())
        owner.addChild(child)
        #endif
        status = 1
        controller = child
        child.view.translatesAutoresizingMaskIntoConstraints = false
        #if os(macOS)
        parent.wantsLayer = true
        child.view.wantsLayer = true
        child.view.layer?.masksToBounds = true
        #else
        child.view.backgroundColor = .clear
        child.view.clipsToBounds = true
        #endif
        parent.addSubview(child.view)
        NSLayoutConstraint.activate([
            child.view.leadingAnchor.constraint(equalTo: parent.leadingAnchor),
            child.view.trailingAnchor.constraint(equalTo: parent.trailingAnchor),
            child.view.topAnchor.constraint(equalTo: parent.topAnchor),
            child.view.bottomAnchor.constraint(equalTo: parent.bottomAnchor),
        ])
        #if os(iOS)
        child.didMove(toParent: owner)
        #endif
        return 0
    }

    static func canvasReady(_ stop: @escaping () -> Void) {
        guard let session = active, session.status == 1 else { return }
        session.stopCanvas = stop
        session.status = 2
    }

    func request(_ data: [UInt8], context: UnsafeMutableRawPointer?, reply: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32, UnsafePointer<UInt8>?, Int) -> Void) {
        let ticket = Unmanaged.passRetained(PaxHostReply(context: context, reply: reply)).toOpaque()
        guard status < 3, let engine else {
            finishHostReply(ticket, 1, nil, 0)
            return
        }
        #if os(macOS)
        PaxCartridgeRuntime.shared.request(engine, data: data, context: ticket, reply: finishHostReply)
        #else
        data.withUnsafeBufferPointer { pax_cartridge_request(engine, $0.baseAddress, Int($0.count), ticket, finishHostReply) }
        #endif
    }

    func close() {
        precondition(Thread.isMainThread)
        guard status < 3 else { return }
        status = 3
        let stop = stopCanvas
        stopCanvas = nil
        stop?()
        #if os(iOS)
        controller?.willMove(toParent: nil)
        #endif
        controller?.view.removeFromSuperview()
        #if os(iOS)
        controller?.removeFromParent()
        #endif
        controller = nil
        // Revoke interrupt routing before resetting the shared native controls.
        NativeInterruptDispatcher.shared.sendData = { _ in }
        NativeInterruptDispatcher.shared.requestKeyboardFocus = nil
        PaxNativeHostState.reset()
        if let engine {
            self.engine = nil
            #if os(macOS)
            PaxViewMacos.PaxEngineContainer.paxEngineContainer = nil
            PaxCartridgeRuntime.shared.deallocEngine(engine)
            #else
            PaxViewIos.PaxEngineContainer.paxEngineContainer = nil
            pax_dealloc_engine(engine)
            #endif
        }
        #if os(macOS)
        Self.cleanupWake(pending: PaxCartridgeRuntime.shared.pumpShutdowns())
        #else
        Self.cleanupWake(pending: pax_pump_shutdowns())
        #endif
    }

    static func cleanupWake(pending: UInt64) {
        guard let session = active, session.status == 3, pending == 0 else { return }
        // Clear the reservation before notifying, permitting a new prepare in
        // the closed callback. The local reference retains this observation.
        active = nil
        session.status = 4
    }

    deinit { release?(context) }
}

// Copy at the ABI boundary, then deliver outside any engine borrow/teardown.
private final class PaxHostReply {
    let context: UnsafeMutableRawPointer?
    let reply: @convention(c) (UnsafeMutableRawPointer?, UInt32, UnsafePointer<UInt8>?, Int) -> Void
    init(context: UnsafeMutableRawPointer?, reply: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32, UnsafePointer<UInt8>?, Int) -> Void) { self.context = context; self.reply = reply }
}
private func finishHostReply(_ context: UnsafeMutableRawPointer?, _ status: UInt32, _ bytes: UnsafePointer<UInt8>?, _ count: Int) {
    let ticket = Unmanaged<PaxHostReply>.fromOpaque(context!).takeRetainedValue()
    let copy = bytes.map { Array(UnsafeBufferPointer(start: $0, count: Int(count))) } ?? Array("application is closed".utf8)
    DispatchQueue.main.async {
        copy.withUnsafeBufferPointer { ticket.reply(ticket.context, status, $0.baseAddress, Int($0.count)) }
    }
}

@_cdecl("pax_host_version") public func paxHostVersion() -> UInt32 { 1 }
@_cdecl("pax_host_prepare") public func paxHostPrepare() -> UnsafeMutableRawPointer? {
    paxHostPrepareWithData(nil, 0)
}
@_cdecl("pax_host_prepare_with_data") public func paxHostPrepareWithData(_ data: UnsafePointer<UInt8>?, _ count: Int) -> UnsafeMutableRawPointer? {
    guard count >= 0, count <= 65536, count == 0 || data != nil else { _ = PaxHostSession.fail("invalid startup data (maximum 64 KiB)"); return nil }
    let bytes = data.map { Array(UnsafeBufferPointer(start: $0, count: Int(count))) } ?? []
    guard let session = PaxHostSession.prepare(data: bytes) else { return nil }
    return Unmanaged.passRetained(session).toOpaque()
}
private func session(_ pointer: UnsafeMutableRawPointer) -> PaxHostSession {
    Unmanaged<PaxHostSession>.fromOpaque(pointer).takeUnretainedValue()
}
@_cdecl("pax_host_observe") public func paxHostObserve(_ pointer: UnsafeMutableRawPointer, _ context: UnsafeMutableRawPointer?,
    _ notify: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32) -> Void,
    _ release: @escaping @convention(c) (UnsafeMutableRawPointer?) -> Void) {
    precondition(Thread.isMainThread)
    session(pointer).observe(context, notify, release)
}
@_cdecl("pax_host_mount") public func paxHostMount(_ pointer: UnsafeMutableRawPointer, _ view: UnsafeMutableRawPointer, _ owner: UnsafeMutableRawPointer?) -> Int32 {
    guard Thread.isMainThread else { return PaxHostSession.fail("Pax mounting requires the main thread") }
    let parent = Unmanaged<AnyObject>.fromOpaque(view).takeUnretainedValue()
    guard let parent = parent as? PaxPlatformView else { return PaxHostSession.fail("Invalid native container type") }
    return session(pointer).mount(parent, owner: owner.map { Unmanaged<AnyObject>.fromOpaque($0).takeUnretainedValue() })
}
@_cdecl("pax_host_request") public func paxHostRequest(_ pointer: UnsafeMutableRawPointer, _ data: UnsafePointer<UInt8>?, _ count: Int, _ context: UnsafeMutableRawPointer?, _ reply: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32, UnsafePointer<UInt8>?, Int) -> Void) {
    precondition(Thread.isMainThread)
    guard count >= 0, count <= 65536, count == 0 || data != nil else {
        let ticket = Unmanaged.passRetained(PaxHostReply(context: context, reply: reply)).toOpaque()
        let error = Array("invalid host request (maximum 64 KiB)".utf8)
        error.withUnsafeBufferPointer { finishHostReply(ticket, 1, $0.baseAddress, Int($0.count)) }
        return
    }
    session(pointer).request(data.map { Array(UnsafeBufferPointer(start: $0, count: Int(count))) } ?? [], context: context, reply: reply)
}
@_cdecl("pax_host_close") public func paxHostClose(_ pointer: UnsafeMutableRawPointer) { session(pointer).close() }
@_cdecl("pax_host_release") public func paxHostRelease(_ pointer: UnsafeMutableRawPointer) {
    precondition(Thread.isMainThread)
    Unmanaged<PaxHostSession>.fromOpaque(pointer).release()
}
@_cdecl("pax_host_error") public func paxHostError(_ buffer: UnsafeMutablePointer<UInt8>?, _ capacity: Int) -> Int {
    let bytes = Array(PaxHostSession.lastError.utf8)
    if let buffer { for index in 0..<min(Int(capacity), bytes.count) { buffer[index] = bytes[index] } }
    return Int(bytes.count)
}
@_cdecl("pax_host_run") public func paxHostRun(_ pointer: UnsafeMutableRawPointer) -> Int32 {
    guard Thread.isMainThread, PaxHostSession.active === session(pointer), session(pointer).status == 0 else {
        return PaxHostSession.fail("Standalone run requires a fresh prepared application on the main thread")
    }
    session(pointer).isStandalone = true
    #if os(macOS)
    pax_app_macosApp.main()
    #else
    pax_app_iosApp.main()
    #endif
    return 0
}

#if os(macOS)
@_cdecl("pax_host_window_options") public func paxHostWindowOptions(_ pointer: UnsafeMutableRawPointer, _ title: UnsafePointer<UInt8>?, _ count: Int, _ width: Double, _ height: Double, _ minWidth: Double, _ minHeight: Double, _ resizable: Bool) -> Int32 {
    guard Thread.isMainThread, session(pointer).status == 0 else { return PaxHostSession.fail("Configure a fresh application on the main thread") }
    guard count >= 0, count <= 65536, count == 0 || title != nil, [width, height, minWidth, minHeight].allSatisfy({ $0.isFinite && $0 >= 0 }), width > 0, height > 0, width >= minWidth, height >= minHeight else { return PaxHostSession.fail("Invalid window dimensions or title") }
    let app = session(pointer)
    if count > 0 { app.windowTitle = String(decoding: UnsafeBufferPointer(start: title!, count: Int(count)), as: UTF8.self) }
    app.windowSize = NSSize(width: width, height: height)
    app.windowMinimum = NSSize(width: minWidth, height: minHeight)
    app.windowResizable = resizable
    return 0
}
private final class PaxStandaloneContainer: NSView {
    weak var session: PaxHostSession?
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if let window { session?.configureWindow(window) }
    }
}
#endif

// Standalone uses exactly the same child hierarchy and containment as embedding.
#if os(macOS)
struct PaxStandaloneView: NSViewRepresentable {
    final class Coordinator { weak var session: PaxHostSession? }
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeNSView(context: Context) -> NSView {
        let container = PaxStandaloneContainer()
        if let session = PaxHostSession.active, session.mount(container, owner: nil) == 0 { context.coordinator.session = session; container.session = session }
        else { print(PaxHostSession.lastError) }
        return container
    }
    func updateNSView(_ view: NSView, context: Context) {}
    static func dismantleNSView(_ view: NSView, coordinator: Coordinator) { coordinator.session?.close() }
}
#else
struct PaxStandaloneView: UIViewControllerRepresentable {
    final class Coordinator { weak var session: PaxHostSession? }
    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIViewController(context: Context) -> UIViewController {
        let owner = UIViewController()
        if let session = PaxHostSession.active, session.mount(owner.view, owner: owner) == 0 { context.coordinator.session = session }
        else { print(PaxHostSession.lastError) }
        return owner
    }
    func updateUIViewController(_ owner: UIViewController, context: Context) {}
    static func dismantleUIViewController(_ owner: UIViewController, coordinator: Coordinator) { coordinator.session?.close() }
}
#endif
