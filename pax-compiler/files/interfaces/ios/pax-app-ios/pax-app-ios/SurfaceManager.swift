import Foundation
import QuartzCore
import FlexBuffers
import Rendering
import PaxCartridge
import UIKit

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

private func withoutImplicitAnimations(_ body: () -> Void) {
    CATransaction.begin()
    CATransaction.setDisableActions(true)
    UIView.performWithoutAnimation {
        body()
    }
    CATransaction.commit()
}

final class SurfaceManager {
    private struct LayerState {
        var surfaceViews: [String: PaxMetalSurfaceView] = [:]
        var lastSignature: String? = nil
    }

    private var layerStates: [UInt32: LayerState] = [:]
    private var lastLayerCount: Int = 0

    func sync(engineContainer: OpaquePointer, rootView: UIView, scale: CGFloat) {
        let layerCount = NativeLayerCountTracker.shared.layerCount
        pax_surface_registry_begin_frame(engineContainer, UInt32(layerCount))

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
            pax_refresh_render_surfaces(engineContainer)
        }
    }

    private func fetchPlan(
        engineContainer: OpaquePointer,
        layerId: UInt32,
        scale: CGFloat
    ) -> LayerCanvasPlan? {
        guard let planQueue = pax_get_layer_canvas_plan(engineContainer, layerId, Float(scale)) else {
            return nil
        }
        defer { pax_dealloc_message_queue(planQueue) }
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
        rootView: UIView,
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
                    withoutImplicitAnimations {
                        surfaceView.frame = frame
                    }
                }

                let pixelWidth = max(1, Int((descriptor.width * Double(scale)).rounded()))
                let pixelHeight = max(1, Int((descriptor.height * Double(scale)).rounded()))
                surfaceView.configure(
                    scale: scale,
                    pixelSize: CGSize(width: pixelWidth, height: pixelHeight)
                )
                let shouldHide = !active
                if surfaceView.isHidden != shouldHide {
                    withoutImplicitAnimations {
                        surfaceView.isHidden = shouldHide
                    }
                }
                let opacity = canvasOpacityMultiplier(for: descriptor.hostSignature)
                if abs(surfaceView.alpha - opacity) > 0.0001
                    || abs(surfaceView.layer.opacity - Float(opacity)) > 0.0001 {
                    withoutImplicitAnimations {
                        surfaceView.alpha = opacity
                        surfaceView.layer.opacity = Float(opacity)
                    }
                }

                if surfaceView.superview !== hostView {
                    withoutImplicitAnimations {
                        hostView.addSubview(surfaceView)
                    }
                }

                descriptor.key.withCString { keyPtr in
                    descriptor.hostSignature.withCString { hostPtr in
                        pax_surface_registry_register_surface(
                            engineContainer,
                            layerId,
                            keyPtr,
                            hostPtr,
                            Float(descriptor.left),
                            Float(descriptor.top),
                            descriptor.replayPriority,
                            Float(descriptor.width),
                            Float(descriptor.height),
                            UInt32(pixelWidth),
                            UInt32(pixelHeight),
                            Float(scale),
                            Float(scale),
                            Unmanaged.passUnretained(surfaceView.metalLayer).toOpaque()
                        )
                    }
                }

                registered.append(descriptor)
            }
        }

        let staleSurfaceIds = state.surfaceViews.keys.filter { !activeIds.contains($0) }
        for id in staleSurfaceIds {
            if let view = state.surfaceViews[id] {
                withoutImplicitAnimations {
                    view.removeFromSuperview()
                }
            }
            state.surfaceViews.removeValue(forKey: id)
        }

        if registered.isEmpty {
            active = false
        }

        pax_surface_registry_set_layer_active(engineContainer, layerId, active)
        return planSignature(layerId: layerId, active: active, surfaces: registered)
    }

    private func hostView(for hostSignature: String, rootView: UIView) -> UIView? {
        if let scrollerId = scrollerId(for: hostSignature) {
            return NativeScrollerHostRegistry.shared.canvasHost(for: scrollerId)
        }
        return rootView
    }

    private func scrollerId(for hostSignature: String) -> UInt32? {
        guard hostSignature.hasPrefix("scroller:") else {
            return nil
        }
        let parts = hostSignature.split(separator: ":")
        guard parts.count == 2 else {
            return nil
        }
        return UInt32(parts[1])
    }

    private func canvasOpacityMultiplier(for hostSignature: String) -> CGFloat {
        guard let scrollerId = scrollerId(for: hostSignature) else {
            return 1.0
        }
        return CGFloat(NativeScrollerHostRegistry.shared.canvasOpacityMultiplier(for: scrollerId))
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

final class PaxMetalSurfaceView: UIView {
    override class var layerClass: AnyClass {
        CAMetalLayer.self
    }

    var metalLayer: CAMetalLayer {
        layer as! CAMetalLayer
    }

    private var appliedScale: CGFloat = 0
    private var appliedPixelSize: CGSize = .zero

    override init(frame: CGRect) {
        super.init(frame: frame)
        commonInit()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        commonInit()
    }

    private func commonInit() {
        backgroundColor = .clear
        isOpaque = false
        isUserInteractionEnabled = false
        metalLayer.framebufferOnly = false
        metalLayer.isOpaque = false
        metalLayer.presentsWithTransaction = false
        metalLayer.colorspace = CGColorSpace(name: CGColorSpace.sRGB)
    }

    func configure(scale: CGFloat, pixelSize: CGSize) {
        if appliedScale != scale {
            contentScaleFactor = scale
            metalLayer.contentsScale = scale
            appliedScale = scale
        }
        if appliedPixelSize != pixelSize {
            metalLayer.drawableSize = pixelSize
            appliedPixelSize = pixelSize
        }
    }
}
