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

// Copied launch metadata. Bundle resources remain the distribution default.
struct PaxNativeExports: Decodable, Equatable {
    var abiVersion: String
    var programId: String
    var initializer: String
    var initWithData: String
    var initEntry: String

    enum CodingKeys: String, CodingKey {
        case abiVersion, programId, initWithData, initEntry
        case initializer = "init"
    }

    func validate() throws {
        for name in [abiVersion, programId, initializer, initWithData, initEntry] {
            guard !name.isEmpty, name.utf8.count <= 128,
                  name.utf8.allSatisfy({ (65...90).contains($0) || (97...122).contains($0) || (48...57).contains($0) || $0 == 95 }) else {
                throw NSError(domain: "PaxCartridgeRuntime", code: 8,
                    userInfo: [NSLocalizedDescriptionKey: "Invalid native program export; rebuild the launcher, cartridge and host together"])
            }
        }
    }
}

struct PaxHostConfiguration: Decodable {
    var cartridge: String?
    var programId: String?
    var nativeExports: PaxNativeExports?
    var assets: [String]?
    var guestBundle: Bool?
    var title: String?
    var entry: String?
    var devSessionDirectory: String?
    var devRegistryFile: String?
    var designServerAddress: String?
    static var current = PaxHostConfiguration()

    var resourceDirectories: [URL] {
        if let assets, guestBundle == true || !assets.isEmpty {
            return assets.map { URL(fileURLWithPath: $0, isDirectory: true) }
        }
        guard let bundle = Bundle.main.url(forResource: "PaxSwiftCartridge_PaxCartridgeAssets", withExtension: "bundle"),
              let resources = Bundle(url: bundle)?.resourceURL else { return [] }
        return [resources]
    }

    func imageURL(_ path: String) -> URL? {
        let relative = path.hasPrefix("assets/") ? String(path.dropFirst(7)) : path
        for directory in resourceDirectories {
            let candidate = directory.appendingPathComponent(relative)
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
        }
        // SwiftPM's processed bundle flattens resources during packaging.
        for directory in resourceDirectories {
            let candidate = directory.appendingPathComponent(URL(fileURLWithPath: path).lastPathComponent)
            if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
        }
        return nil
    }
}

@_cdecl("pax_host_configure") public func paxHostConfigure(_ bytes: UnsafePointer<UInt8>?, _ count: Int) -> Int32 {
    guard Thread.isMainThread, PaxHostSession.active == nil else {
        return PaxHostSession.fail("Configure Pax on the main thread after the previous mount has closed")
    }
    guard count >= 0, count <= 1_048_576, count == 0 || bytes != nil else {
        return PaxHostSession.fail("Invalid Pax launch metadata")
    }
    do {
        let data = count == 0 ? Data() : Data(bytes: bytes!, count: count)
        let configuration = try JSONDecoder().decode(PaxHostConfiguration.self, from: data)
        PaxHostConfiguration.current = configuration
        return 0
    } catch { return PaxHostSession.fail("Invalid Pax launch metadata: \(error)") }
}

// This module is the host side of a versioned C boundary. It never receives a
// Rust root, Property, runtime handle or closure from the application cartridge.
final class PaxHostSession {
    // The legacy configure/prepare pair reserves one outer application.
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
    var engine: OpaquePointer?
    #if os(macOS)
    let nativeState = PaxNativeHostState()
    let runtime: PaxCartridgeRuntime
    let configuration: PaxHostConfiguration

    private init(configuration: PaxHostConfiguration) {
        self.configuration = configuration
        runtime = PaxCartridgeRuntime(configuration: configuration)
        windowTitle = configuration.title
    }

    private var guests: [String: PaxHostConfiguration] = [:]
    private var sessionDirectory: URL?

    func allowSessionDirectory(_ url: URL) -> Int32 {
        guard status == 0, sessionDirectory == nil,
              (try? url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true else {
            return Self.fail("Configure one existing guest descriptor directory before mounting")
        }
        sessionDirectory = url.standardizedFileURL
        enableGuestMounting()
        return 0
    }

    private struct GuestDescriptor: Decodable {
        let bundle: String
        let devSessionDirectory: String
        let designServerAddress: String
        let devRegistryFile: String
    }

    private func configuration(for name: String) -> PaxHostConfiguration? {
        if let existing = guests[name] { return existing }
        guard let sessionDirectory, (1...128).contains(name.utf8.count),
              name.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || $0 == 45 || $0 == 95 })
        else { return nil }
        do {
            let url = sessionDirectory.appendingPathComponent(name + ".json")
            let descriptor = try JSONDecoder().decode(GuestDescriptor.self, from: Data(contentsOf: url))
            guard var config = guestConfiguration(URL(fileURLWithPath: descriptor.bundle)) else { return nil }
            config.devSessionDirectory = descriptor.devSessionDirectory
            config.designServerAddress = descriptor.designServerAddress
            config.devRegistryFile = descriptor.devRegistryFile
            return config
        } catch {
            NSLog("Pax guest %@: %@", name, error.localizedDescription)
            return nil
        }
    }

    private func enableGuestMounting() {
        nativeState.mountInlineSession = { [weak self] name, view in self?.mountGuest(name, in: view) }
    }

    private var onClosed: (() -> Void)?
    private var mountedGuests: [ObjectIdentifier: PaxHostSession] = [:]

    func registerGuest(name: String, bundleURL: URL) -> Int32 {
        guard status == 0, !name.isEmpty, guests[name] == nil else {
            return Self.fail("Register a unique, nonempty guest name before mounting the host")
        }
        guard var config = guestConfiguration(bundleURL) else { return -1 }
        if ProcessInfo.processInfo.environment["PAX_HOSTED_GUEST_APP"].map({ URL(fileURLWithPath: $0).standardizedFileURL }) == bundleURL.standardizedFileURL {
            config.devSessionDirectory = ProcessInfo.processInfo.environment["PAX_HOSTED_DEV_SESSION_DIR"]
            config.designServerAddress = ProcessInfo.processInfo.environment["PAX_HOSTED_DESIGN_SERVER_ADDR"]
            config.devRegistryFile = ProcessInfo.processInfo.environment["PAX_HOSTED_DEV_REGISTRY_FILE"]
        }
        guests[name] = config
        enableGuestMounting()
        return 0
    }

    private func guestConfiguration(_ bundleURL: URL) -> PaxHostConfiguration? {
        guard let bundle = Bundle(url: bundleURL), let frameworks = bundle.privateFrameworksURL else {
            _ = Self.fail("Expected a Pax macOS application bundle"); return nil
        }
        let cartridge = frameworks.appendingPathComponent("PaxCartridge.framework/PaxCartridge")
        guard FileManager.default.isReadableFile(atPath: cartridge.path) else {
            _ = Self.fail("Missing PaxCartridge.framework in " + bundleURL.path); return nil
        }
        var config = PaxHostConfiguration()
        config.guestBundle = true
        config.cartridge = cartridge.path
        if let resources = bundle.url(forResource: "PaxSwiftCartridge_PaxCartridgeAssets", withExtension: "bundle"),
           let directory = Bundle(url: resources)?.resourceURL {
            config.assets = [directory.path]
        } else { config.assets = [] }
        return config
    }

    private func mountGuest(_ name: String, in view: NSView) -> (() -> Void)? {
        guard status < 3, let configuration = configuration(for: name) else { return nil }
        // A removed frame can be reinserted before its cartridge finishes stopping.
        // Reconciliation runs again when that exact image acknowledges cleanup.
        let image = configuration.cartridge
        guard !mountedGuests.values.contains(where: { $0.configuration.cartridge == image }),
              !Self.closing.values.contains(where: { $0.configuration.cartridge == image }) else { return nil }
        guard let guest = Self.prepareMac(configuration: configuration, data: []) else {
            NSLog("Pax guest %@: %@", name, Self.lastError)
            return nil
        }
        guard guest.mount(view, owner: nil) == 0 else { guest.close(); return nil }
        guest.onClosed = { [weak self] in
            if self?.sessionDirectory != nil, let directory = configuration.devSessionDirectory {
                let marker = URL(fileURLWithPath: directory).appendingPathComponent("host-closed")
                try? Data().write(to: marker, options: .atomic)
            }
            DispatchQueue.main.async {
                guard let self, self.status < 3 else { return }
                self.nativeState.invalidation.invalidate()
            }
        }
        let id = ObjectIdentifier(guest)
        mountedGuests[id] = guest
        return { [weak self] in self?.mountedGuests.removeValue(forKey: id)?.close() }
    }

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
        #if os(macOS)
        var configuration = PaxHostConfiguration.current
        configuration.devSessionDirectory = ProcessInfo.processInfo.environment["PAX_DEV_SESSION_DIR"]
        configuration.devRegistryFile = ProcessInfo.processInfo.environment["PAX_DEV_REGISTRY_FILE"]
        return prepareMac(configuration: configuration, data: data, standaloneCandidate: true)
        #else
        let session = PaxHostSession()
        active = session
        guard pax_cartridge_abi_version() == 3 else {
            active = nil
            _ = fail("Incompatible Pax cartridge ABI")
            return nil
        }
        pax_set_shutdown_waker(1) { _ in
            DispatchQueue.main.async { PaxHostSession.cleanupWake(pending: pax_pump_shutdowns()) }
        }
        let entry = Array((PaxHostConfiguration.current.entry ?? "").utf8)
        session.engine = entry.withUnsafeBufferPointer { entry in
            data.withUnsafeBufferPointer { data in
                pax_init_entry(0, 0, entry.baseAddress, entry.count, data.baseAddress, data.count)
            }
        }
        PaxViewIos.PaxEngineContainer.paxEngineContainer = session.engine
        guard session.engine != nil else {
            _ = fail("Cartridge initialization failed; see the initializer diagnostic")
            session.close()
            return nil
        }
        return session
        #endif
    }

    #if os(macOS)
    static func prepareMac(configuration: PaxHostConfiguration, data: [UInt8], standaloneCandidate: Bool = false) -> PaxHostSession? {
        precondition(Thread.isMainThread)
        let session = PaxHostSession(configuration: configuration)
        if standaloneCandidate { active = session }
        session.runtime.didPumpShutdowns = { [weak session] pending in session?.finishCleanup(pending: pending) }
        do {
            session.engine = try session.runtime.prepareEngine(data: data)
            return session
        } catch {
            _ = fail(error.localizedDescription)
            session.close()
            return nil
        }
    }
    #endif

    func observe(_ context: UnsafeMutableRawPointer?, _ notify: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32) -> Void,
                 _ release: @escaping @convention(c) (UnsafeMutableRawPointer?) -> Void) {
        self.release?(self.context)
        self.context = context
        self.notify = notify
        self.release = release
        notify(context, status)
    }

    func mount(_ parent: PaxPlatformView, owner: AnyObject?) -> Int32 {
        guard Thread.isMainThread, status == 0 else {
            return Self.fail("Prepared Pax application must be mounted once on the main thread")
        }
        #if os(macOS)
        let child = NSHostingController(rootView: PaxViewMacos(session: self))
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
        session.markReady(stop)
    }

    func markReady(_ stop: @escaping () -> Void) {
        guard status == 1 else { return }
        stopCanvas = stop
        status = 2
    }

    func request(_ data: [UInt8], context: UnsafeMutableRawPointer?, reply: @escaping @convention(c) (UnsafeMutableRawPointer?, UInt32, UnsafePointer<UInt8>?, Int) -> Void) {
        let ticket = Unmanaged.passRetained(PaxHostReply(context: context, reply: reply)).toOpaque()
        guard status < 3, let engine else {
            finishHostReply(ticket, 1, nil, 0)
            return
        }
        #if os(macOS)
        runtime.request(engine, data: data, context: ticket, reply: finishHostReply)
        #else
        data.withUnsafeBufferPointer { pax_cartridge_request(engine, $0.baseAddress, Int($0.count), ticket, finishHostReply) }
        #endif
    }

    func close() {
        precondition(Thread.isMainThread)
        guard status < 3 else { return }
        status = 3
        #if os(macOS)
        nativeState.mountInlineSession = nil
        let children = mountedGuests
        mountedGuests.removeAll()
        for child in children.values { child.close() }
        #endif
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
        #if os(macOS)
        nativeState.dispatcher.sendData = nil
        nativeState.dispatcher.requestKeyboardFocus = nil
        nativeState.dispatcher.wheelInterrupt = nil
        nativeState.dispatcher.convertWindowPointToPax = nil
        nativeState.clear()
        #else
        // Revoke interrupt routing before resetting the shared native controls.
        NativeInterruptDispatcher.shared.sendData = { _ in }
        NativeInterruptDispatcher.shared.requestKeyboardFocus = nil
        PaxNativeHostState.reset()
        #endif
        if let engine {
            self.engine = nil
            #if os(macOS)
            runtime.deallocEngine(engine)
            #else
            PaxViewIos.PaxEngineContainer.paxEngineContainer = nil
            pax_dealloc_engine(engine)
            #endif
        }
        #if os(macOS)
        Self.closing[ObjectIdentifier(self)] = self
        finishCleanup(pending: runtime.pumpShutdowns())
        #else
        Self.cleanupWake(pending: pax_pump_shutdowns())
        #endif
    }

    #if os(macOS)
    private static var closing: [ObjectIdentifier: PaxHostSession] = [:]

    private func finishCleanup(pending: UInt64) {
        guard status == 3, pending == 0 else { return }
        runtime.releaseReservation()
        if Self.active === self { Self.active = nil }
        Self.closing.removeValue(forKey: ObjectIdentifier(self))
        status = 4
        onClosed?()
        onClosed = nil
    }
    #endif

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

@_cdecl("pax_host_version") public func paxHostVersion() -> UInt32 { 2 }
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
@_cdecl("pax_host_allow_inline_session_directory") public func paxHostAllowInlineSessionDirectory(
    _ pointer: UnsafeMutableRawPointer, _ path: UnsafePointer<UInt8>?, _ length: Int
) -> Int32 {
    guard Thread.isMainThread, (1...65536).contains(length), let path,
          let directory = String(bytes: UnsafeBufferPointer(start: path, count: length), encoding: .utf8)
    else { return PaxHostSession.fail("Invalid guest descriptor directory") }
    return session(pointer).allowSessionDirectory(URL(fileURLWithPath: directory, isDirectory: true))
}

@_cdecl("pax_host_register_inline_app") public func paxHostRegisterInlineApp(
    _ pointer: UnsafeMutableRawPointer, _ name: UnsafePointer<UInt8>?, _ nameLength: Int,
    _ path: UnsafePointer<UInt8>?, _ pathLength: Int
) -> Int32 {
    guard Thread.isMainThread, (1...1024).contains(nameLength), (1...65536).contains(pathLength),
          let name, let path,
          let key = String(bytes: UnsafeBufferPointer(start: name, count: nameLength), encoding: .utf8),
          let bundle = String(bytes: UnsafeBufferPointer(start: path, count: pathLength), encoding: .utf8)
    else { return PaxHostSession.fail("Invalid guest name or bundle path") }
    return session(pointer).registerGuest(name: key, bundleURL: URL(fileURLWithPath: bundle))
}

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
