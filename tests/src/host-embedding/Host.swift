// A caller-owned application: its controls, window/controller and event loop
// survive Pax disposal. Compile against the framework from a fixture build.
import SwiftUI
#if os(macOS)
import AppKit
typealias ContainerView = NSView
#else
import UIKit
typealias ContainerView = UIView
#endif

final class MountModel: ObservableObject {
    @Published var status = "Not mounted"
    @Published var text = "This input belongs to the host"
    @Published var clicks = 0
    @Published var compact = false
    @Published var collapsed = false
    weak var container: ContainerView?
    weak var controller: AnyObject?
    private var handle: UnsafeMutableRawPointer?

    func mount() {
        guard handle == nil, let container else { return }
        let startup = Array("7".utf8)
        guard let prepared = startup.withUnsafeBufferPointer({ pax_host_prepare_with_data($0.baseAddress, $0.count) }) else { status = "Prepare rejected"; return }
        handle = prepared
        pax_host_observe(prepared, Unmanaged.passRetained(self).toOpaque(), { context, phase in
            let model = Unmanaged<MountModel>.fromOpaque(context!).takeUnretainedValue()
            let labels = ["Prepared", "Mounting", "Ready", "Closing", "Closed: cleanup acknowledged"]
            model.status = labels[Int(phase)]
            print("PAX_HOST_FIXTURE \(model.status)")
        }, { context in Unmanaged<MountModel>.fromOpaque(context!).release() })
        let owner = controller.map { Unmanaged.passUnretained($0).toOpaque() }
        if pax_host_mount(prepared, Unmanaged.passUnretained(container).toOpaque(), owner) != 0 {
            close()
            status = "Mount failed"
        }
    }
    func request() {
        guard let handle else { return }
        let bytes = Array("increment".utf8)
        bytes.withUnsafeBufferPointer {
            pax_host_request(handle, $0.baseAddress, $0.count, Unmanaged.passRetained(self).toOpaque()) { context, status, data, count in
                let model = Unmanaged<MountModel>.fromOpaque(context!).takeRetainedValue()
                let bytes = data.map { Array(UnsafeBufferPointer(start: $0, count: count)) } ?? []
                model.status = "Cartridge \(status == 0 ? "replied" : "rejected"): \(String(decoding: bytes, as: UTF8.self))"
            }
        }
    }
    func failInitialization() {
        guard handle == nil else { status = "Close Pax first"; return }
        let invalid = Array("invalid counter".utf8)
        if let unexpected = invalid.withUnsafeBufferPointer({ pax_host_prepare_with_data($0.baseAddress, $0.count) }) {
            pax_host_close(unexpected); pax_host_release(unexpected)
            status = "Unexpected initialization success"
        } else { status = "Initializer error reported; cleanup continues" }
    }
    func duplicate() {
        if let unexpected = pax_host_prepare() {
            pax_host_close(unexpected)
            pax_host_release(unexpected)
            status = "Unexpected second prepare"
        } else { status = "Second mount rejected" }
    }
    func close() {
        guard let handle else { return }
        self.handle = nil
        pax_host_close(handle)
        pax_host_release(handle)
    }
    deinit { close() }
}

#if os(macOS)
struct HostRegion: NSViewRepresentable {
    let model: MountModel
    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        model.container = view
        return view
    }
    func updateNSView(_ view: NSView, context: Context) {}
    static func dismantleNSView(_ view: NSView, coordinator: MountModel) { coordinator.close() }
    func makeCoordinator() -> MountModel { model }
}
#else
struct HostRegion: UIViewControllerRepresentable {
    let model: MountModel
    func makeUIViewController(context: Context) -> UIViewController {
        let owner = UIViewController()
        model.container = owner.view
        model.controller = owner
        return owner
    }
    func updateUIViewController(_ owner: UIViewController, context: Context) {}
    static func dismantleUIViewController(_ owner: UIViewController, coordinator: MountModel) { coordinator.close() }
    func makeCoordinator() -> MountModel { model }
}
#endif

@main
struct HostEmbeddingProof: App {
    @StateObject private var model = MountModel()
    var body: some Scene {
        WindowGroup("Pax host embedding") {
            VStack(alignment: .leading, spacing: 12) {
                Text("Caller-owned host").font(.title2)
                TextField("Host text", text: $model.text).textFieldStyle(.roundedBorder)
                HStack {
                    Button("Host clicks: \(model.clicks)") { model.clicks += 1 }
                    Button("Mount Pax") { model.mount() }
                    Button("Close Pax") { model.close() }
                }
                HStack {
                    Button("Try second mount") { model.duplicate() }
                    Button("Request increment") { model.request() }
                }
                HStack {
                    Button("Resize region") { model.compact.toggle() }
                    Button("Collapse region") { model.collapsed.toggle() }
                    Button("Fail initialization") { model.failInitialization() }
                }
                Text(model.status).accessibilityIdentifier("mount-status")
                HostRegion(model: model).frame(maxWidth: .infinity, maxHeight: .infinity)
                    .frame(height: model.collapsed ? 0 : (model.compact ? 260 : nil))
                    .border(Color.gray)
            }.padding()
            #if os(macOS)
            .frame(minWidth: 920, minHeight: 780)
            #endif
        }
    }
}
