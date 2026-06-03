import Foundation
import QuartzCore
import FlexBuffers
import Rendering
import PaxCartridge
import AppKit

private let disabledSurfaceLayerActions: [String: CAAction] = [
    "anchorPoint": NSNull(),
    "backgroundColor": NSNull(),
    "bounds": NSNull(),
    "contents": NSNull(),
    "contentsGravity": NSNull(),
    "contentsScale": NSNull(),
    "drawableSize": NSNull(),
    "frame": NSNull(),
    "hidden": NSNull(),
    "opacity": NSNull(),
    "position": NSNull(),
    "sublayers": NSNull(),
    "transform": NSNull(),
]

private func disableSurfaceLayerImplicitActions(_ layer: CALayer?) {
    layer?.actions = disabledSurfaceLayerActions
}

private struct SurfaceCanvasDescriptor {
    let id: String
    let key: String
    let left: Double
    let top: Double
    let width: Double
    let height: Double
    let replayPriority: Int32
    let surfaceSignature: String
    let transformSignature: String
    let hostSignature: String

    init?(fb: FlxbReference) {
        guard let id = fb["id"]?.asString, let key = fb["key"]?.asString else {
            return nil
        }
        guard let left = readDouble(fb["left"]),
              let top = readDouble(fb["top"]),
              let width = readDouble(fb["width"]),
              let height = readDouble(fb["height"]) else {
            return nil
        }
        self.id = id
        self.key = key
        self.left = left
        self.top = top
        self.width = width
        self.height = height
        self.replayPriority = Int32(readDouble(fb["replayPriority"]) ?? 0)
        self.surfaceSignature = fb["surfaceSignature"]?.asString ?? ""
        self.transformSignature = fb["transformSignature"]?.asString ?? ""
        self.hostSignature = fb["hostSignature"]?.asString ?? "root"
    }
}

private struct LayerCanvasPlan {
    let layerId: UInt32
    let active: Bool
    let surfaces: [SurfaceCanvasDescriptor]

    init?(fb: FlxbReference) {
        let layerIdValue: UInt64
        if let value = fb["layerId"]?.asUInt64 {
            layerIdValue = value
        } else if let value = fb["layer_id"]?.asUInt64 {
            layerIdValue = value
        } else if let value = fb["layerId"]?.asInt {
            layerIdValue = UInt64(truncatingIfNeeded: value)
        } else {
            return nil
        }
        self.layerId = UInt32(truncatingIfNeeded: layerIdValue)
        self.active = fb["active"]?.asBool ?? false
        var surfaces: [SurfaceCanvasDescriptor] = []
        if let vector = fb["surfaces"]?.asVector {
            for item in vector.makeIterator() {
                if let descriptor = SurfaceCanvasDescriptor(fb: item) {
                    surfaces.append(descriptor)
                }
            }
        }
        self.surfaces = surfaces
    }
}

private func readDouble(_ fb: FlxbReference?) -> Double? {
    if let value = fb?.asDouble {
        return value
    }
    if let value = fb?.asFloat {
        return Double(value)
    }
    if let value = fb?.asInt {
        return Double(value)
    }
    return nil
}

final class SurfaceManager {
    private struct LayerState {
        var surfaceViews: [String: PaxMetalSurfaceView] = [:]
        var lastSignature: String? = nil
    }

    private var layerStates: [UInt32: LayerState] = [:]
    private var lastLayerCount: Int = 0

    func reset() {
        for state in layerStates.values {
            for view in state.surfaceViews.values {
                view.removeFromSuperview()
            }
        }
        layerStates.removeAll()
        lastLayerCount = 0
    }

    func sync(engineContainer: OpaquePointer, rootView: NSView, scale: CGFloat) {
        let layerCount = NativeLayerCountTracker.shared.layerCount
        PaxCartridgeRuntime.shared.surfaceRegistryBeginFrame(
            engineContainer,
            layerCount: UInt32(layerCount)
        )

        var needsRefresh = layerCount != lastLayerCount

        for layerIndex in 0..<layerCount {
            let layerId = UInt32(layerIndex)
            let plan = fetchPlan(engineContainer: engineContainer, layerId: layerId, scale: scale)
            var state = layerStates[layerId] ?? LayerState()
            let signature = syncLayer(
                engineContainer: engineContainer,
                layerId: layerId,
                plan: plan,
                state: &state,
                rootView: rootView,
                scale: scale
            )
            if signature != state.lastSignature {
                needsRefresh = true
            }
            state.lastSignature = signature
            layerStates[layerId] = state
        }

        if lastLayerCount > layerCount {
            let staleLayerIds = layerStates.keys.filter { Int($0) >= layerCount }
            for layerId in staleLayerIds {
                if let state = layerStates[layerId] {
                    for view in state.surfaceViews.values {
                        view.removeFromSuperview()
                    }
                }
                layerStates.removeValue(forKey: layerId)
                needsRefresh = true
            }
        }

        lastLayerCount = layerCount

        if needsRefresh {
            PaxCartridgeRuntime.shared.refreshRenderSurfaces(engineContainer)
        }
    }

    private func fetchPlan(
        engineContainer: OpaquePointer,
        layerId: UInt32,
        scale: CGFloat
    ) -> LayerCanvasPlan? {
        guard let planQueue = PaxCartridgeRuntime.shared.getLayerCanvasPlan(
            engineContainer,
            layerId: layerId,
            scale: Float(scale)
        ) else {
            return nil
        }
        defer { PaxCartridgeRuntime.shared.deallocMessageQueue(planQueue) }
        let queue = planQueue.pointee
        guard let dataPtr = queue.data_ptr else {
            return nil
        }
        let data = Data(bytes: dataPtr, count: Int(queue.length))
        guard let root = FlexBuffer.decode(data: data) else {
            return nil
        }
        return LayerCanvasPlan(fb: root)
    }

    private func syncLayer(
        engineContainer: OpaquePointer,
        layerId: UInt32,
        plan: LayerCanvasPlan?,
        state: inout LayerState,
        rootView: NSView,
        scale: CGFloat
    ) -> String {
        var registered: [SurfaceCanvasDescriptor] = []
        var active = plan?.active ?? false
        var activeIds = Set<String>()

        if let plan {
            for descriptor in plan.surfaces {
                guard let hostView = hostView(for: descriptor.hostSignature, rootView: rootView) else {
                    continue
                }
                let surfaceView = state.surfaceViews[descriptor.id] ?? PaxMetalSurfaceView()
                state.surfaceViews[descriptor.id] = surfaceView
                activeIds.insert(descriptor.id)

                let frame = CGRect(
                    x: descriptor.left,
                    y: descriptor.top,
                    width: descriptor.width,
                    height: descriptor.height
                )
                if surfaceView.frame != frame {
                    surfaceView.frame = frame
                }

                let pixelWidth = max(1, Int((descriptor.width * Double(scale)).rounded()))
                let pixelHeight = max(1, Int((descriptor.height * Double(scale)).rounded()))
                surfaceView.configure(
                    scale: scale,
                    pixelSize: CGSize(width: pixelWidth, height: pixelHeight)
                )
                surfaceView.isHidden = !active

                if surfaceView.superview !== hostView {
                    hostView.addSubview(surfaceView)
                }

                descriptor.key.withCString { keyPtr in
                    descriptor.hostSignature.withCString { hostPtr in
                        PaxCartridgeRuntime.shared.surfaceRegistryRegisterSurface(
                            engineContainer,
                            layerId: layerId,
                            key: keyPtr,
                            hostSignature: hostPtr,
                            originX: Float(descriptor.left),
                            originY: Float(descriptor.top),
                            replayPriority: descriptor.replayPriority,
                            logicalWidth: Float(descriptor.width),
                            logicalHeight: Float(descriptor.height),
                            surfaceWidth: UInt32(pixelWidth),
                            surfaceHeight: UInt32(pixelHeight),
                            dprX: Float(scale),
                            dprY: Float(scale),
                            layerPointer: Unmanaged.passUnretained(surfaceView.metalLayer).toOpaque()
                        )
                    }
                }

                registered.append(descriptor)
            }
        }

        let staleSurfaceIds = state.surfaceViews.keys.filter { !activeIds.contains($0) }
        for id in staleSurfaceIds {
            if let view = state.surfaceViews[id] {
                view.removeFromSuperview()
            }
            state.surfaceViews.removeValue(forKey: id)
        }

        if registered.isEmpty {
            active = false
        }

        PaxCartridgeRuntime.shared.surfaceRegistrySetLayerActive(
            engineContainer,
            layerId: layerId,
            active: active
        )
        return planSignature(layerId: layerId, active: active, surfaces: registered)
    }

    private func hostView(for hostSignature: String, rootView: NSView) -> NSView? {
        if hostSignature.hasPrefix("scroller:") {
            let parts = hostSignature.split(separator: ":")
            if parts.count == 2, let id = UInt32(parts[1]) {
                return NativeScrollerHostRegistry.shared.canvasHost(for: id)
            }
            return nil
        }
        return rootView
    }

    private func planSignature(
        layerId: UInt32,
        active: Bool,
        surfaces: [SurfaceCanvasDescriptor]
    ) -> String {
        var parts: [String] = [
            String(layerId),
            active ? "1" : "0",
            String(surfaces.count),
        ]
        for surface in surfaces {
            parts.append(
                "\(surface.id)|\(surface.left),\(surface.top),\(surface.width),\(surface.height)"
                + "|\(surface.surfaceSignature)|\(surface.transformSignature)|\(surface.hostSignature)"
            )
        }
        return parts.joined(separator: ";")
    }
}

final class PaxMetalSurfaceView: NSView {
    private var appliedScale: CGFloat = 0
    private var appliedPixelSize: CGSize = .zero
    override var preservesContentDuringLiveResize: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? {
        nil
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        commonInit()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        commonInit()
    }

    override func makeBackingLayer() -> CALayer {
        CAMetalLayer()
    }

    private func commonInit() {
        wantsLayer = true
        layerContentsRedrawPolicy = .duringViewResize
        layer?.backgroundColor = NSColor.clear.cgColor
        layer?.anchorPoint = CGPoint(x: 0.0, y: 0.0)
        layer?.contentsGravity = .topLeft
        layer?.needsDisplayOnBoundsChange = true
        disableSurfaceLayerImplicitActions(layer)
        if let metalLayer = layer as? CAMetalLayer {
            configureMetalLayer(metalLayer)
        }
    }

    var metalLayer: CAMetalLayer {
        guard let metalLayer = layer as? CAMetalLayer else {
            let layer = CAMetalLayer()
            configureMetalLayer(layer)
            self.layer = layer
            return layer
        }
        return metalLayer
    }

    private func configureMetalLayer(_ metalLayer: CAMetalLayer) {
        metalLayer.framebufferOnly = false
        metalLayer.isOpaque = false
        metalLayer.presentsWithTransaction = false
        metalLayer.colorspace = CGColorSpace(name: CGColorSpace.sRGB)
        metalLayer.contentsGravity = .topLeft
        metalLayer.needsDisplayOnBoundsChange = true
        disableSurfaceLayerImplicitActions(metalLayer)
    }

    func configure(scale: CGFloat, pixelSize: CGSize) {
        if appliedScale != scale {
            metalLayer.contentsScale = scale
            appliedScale = scale
        }
        if appliedPixelSize != pixelSize {
            metalLayer.drawableSize = pixelSize
            appliedPixelSize = pixelSize
        }
    }
}
