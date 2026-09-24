//
//  ContentView.swift
//  interface
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI
import Foundation
import QuartzCore
import FlexBuffers
import Messages
import Rendering
import PaxCartridgeAssets
import PaxCartridge

private func registerPaxFontsIfNeeded() {
    let nestedBundleURL = Bundle.main.url(
        forResource: "PaxSwiftCartridge_PaxCartridgeAssets",
        withExtension: "bundle"
    )!

    let resourceBundle = Bundle(url: nestedBundleURL)!
    let resourceURL = resourceBundle.resourceURL!
    let fontFileExtensions: Set<String> = ["ttf", "otf"]

    let enumerator = FileManager.default.enumerator(
        at: resourceURL,
        includingPropertiesForKeys: nil,
        options: [.skipsHiddenFiles]
    )
    while let fileURL = enumerator?.nextObject() as? URL {
        let fileExtension = fileURL.pathExtension.lowercased()
        if fontFileExtensions.contains(fileExtension) {
            let fontDescriptors = CTFontManagerCreateFontDescriptorsFromURL(fileURL as CFURL) as! [CTFontDescriptor]
            if let fontDescriptor = fontDescriptors.first,
               let postscriptName = CTFontDescriptorCopyAttribute(fontDescriptor, kCTFontNameAttribute) as? String,
               let fontFamily = CTFontDescriptorCopyAttribute(fontDescriptor, kCTFontFamilyNameAttribute) as? String {
                if !PaxFont.isFontRegistered(fontFamily: postscriptName) {
                    var errorRef: Unmanaged<CFError>?
                    if !CTFontManagerRegisterFontsForURL(fileURL as CFURL, .process, &errorRef) {
                        print("Error registering font: \(fontFamily) - PostScript name: \(postscriptName) - \(String(describing: errorRef))")
                    } else {
                        PaxFont.markFontRegistered(fontFamily: postscriptName)
                        PaxFont.markFontRegistered(fontFamily: fontFamily)
                    }
                } else {
                    PaxFont.markFontRegistered(fontFamily: postscriptName)
                    PaxFont.markFontRegistered(fontFamily: fontFamily)
                }
            }
        }
    }
}

private func sendInterruptToEngine(data: Data) {
    guard let engineContainer = PaxViewIos.PaxEngineContainer.paxEngineContainer else {
        return
    }

    data.withUnsafeBytes { ptr in
        var ffi_container = InterruptBuffer(data_ptr: ptr.baseAddress!, length: UInt64(ptr.count))
        withUnsafePointer(to: &ffi_container) { ffi_container_ptr in
            pax_interrupt(engineContainer, ffi_container_ptr)
        }
    }
}

struct PaxViewIos: View {
    init() {
        registerPaxFontsIfNeeded()
        NativeInterruptDispatcher.shared.sendData = sendInterruptToEngine
        PaxMotionSensorBridge.shared.start()
    }

    func canvasView(size: CGSize) -> some View {
        PaxCanvasViewRepresentable()
            .frame(width: size.width, height: size.height, alignment: .topLeading)
    }

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .topLeading) {
                self.canvasView(size: proxy.size)
                NativeRenderingLayer()
                    .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
            }
            .frame(width: proxy.size.width, height: proxy.size.height, alignment: .topLeading)
        }
        .ignoresSafeArea()
        .onAppear {
            NativeInterruptDispatcher.shared.sendData = sendInterruptToEngine
            registerPaxFontsIfNeeded()
            PaxMotionSensorBridge.shared.start()
        }
        .onDisappear {
            PaxMotionSensorBridge.shared.stop()
        }
    }

    class PaxEngineContainer {
        static var paxEngineContainer : OpaquePointer? = nil
    }

    

    struct PaxCanvasViewRepresentable: UIViewRepresentable {
        typealias UIViewType = PaxCanvasViewIos

        func makeUIView(context: Context) -> PaxCanvasViewIos {
            registerPaxFontsIfNeeded()
            NativeInterruptDispatcher.shared.sendData = sendInterruptToEngine
            PaxMotionSensorBridge.shared.start()
            let view = PaxCanvasViewIos()
            return view
        }

        func updateUIView(_ uiView: PaxCanvasViewIos, context: Context) {
        }
    }


    class PaxCanvasViewIos: UIView, NativeMessageHandling {
        let textElements = TextElements.singleton
        let frameElements = FrameElements.singleton
        let scrollerElements = ScrollerElements.singleton
        let buttonElements = ButtonElements.singleton
        let photoPickerElements = PhotoPickerElements.singleton
        let checkboxElements = CheckboxElements.singleton
        let nativeImageElements = NativeImageElements.singleton
        let youtubeVideoElements = YoutubeVideoElements.singleton
        let dropdownElements = DropdownElements.singleton
        let radioListElements = RadioListElements.singleton
        let sliderElements = SliderElements.singleton
        let textboxElements = TextboxElements.singleton
        let eventBlockerElements = EventBlockerElements.singleton
        let glassSurfaceElements = GlassSurfaceElements.singleton
        private var displayLink: CADisplayLink?
        private var previousViewportSize: CGSize = .zero
        private var publishedSafeAreaInsets: UIEdgeInsets?
        private let viewportSizeEpsilon: CGFloat = 0.5
        private let surfaceManager = SurfaceManager()
        private var lastTouchPositions: [ObjectIdentifier: CGPoint] = [:]
        private let frameInstrumentationEnabled = PaxCanvasViewIos.frameInstrumentationFlagEnabled()
        private var frameInstrumentation = PaxFrameInstrumentation()
        private var activeTapTouchKey: ObjectIdentifier?
        private var activeTapStartPosition: CGPoint?
        private let tapMovementTolerance: CGFloat = 10.0

        override init(frame: CGRect) {
            super.init(frame: frame)
            configureView()
        }

        required init?(coder: NSCoder) {
            super.init(coder: coder)
            configureView()
        }

        private func configureView() {
            isOpaque = false
            isMultipleTouchEnabled = true
            installNativeInterruptDispatcher()
            createDisplayLink()
        }

        override func didMoveToWindow() {
            super.didMoveToWindow()
            installNativeInterruptDispatcher()
        }

        private func installNativeInterruptDispatcher() {
            NativeInterruptDispatcher.shared.sendData = sendInterruptToEngine
            NativeInterruptDispatcher.shared.convertWindowPointToPax = { [weak self] point, window in
                guard let self else {
                    return nil
                }
                if let window, self.window !== window {
                    return nil
                }
                return self.convert(point, from: window)
            }
        }


        private var requestAnimationFrameQueue: [() -> Void] = []

        private func processRequestAnimationFrameQueue() {
            while !requestAnimationFrameQueue.isEmpty {
                let closure = requestAnimationFrameQueue.removeFirst()
                closure()
            }
        }

        func requestAnimationFrame(_ closure: @escaping () -> Void) {
            requestAnimationFrameQueue.append(closure)
        }

        private func touchStorageKey(for touch: UITouch) -> ObjectIdentifier {
            ObjectIdentifier(touch)
        }

        private func touchIdentifier(for touch: UITouch) -> Int64 {
            Int64(bitPattern: UInt64(UInt(bitPattern: Unmanaged.passUnretained(touch).toOpaque())))
        }

        private func sortedTouches(_ touches: some Sequence<UITouch>) -> [UITouch] {
            touches.sorted { touchIdentifier(for: $0) < touchIdentifier(for: $1) }
        }

        private func orderedActiveTouches(changedTouches: Set<UITouch>, event: UIEvent?) -> [UITouch] {
            let changedTouchKeys = Set(changedTouches.map(touchStorageKey(for:)))
            let activeTouches = event?.allTouches?.filter { touch in
                touch.phase != .ended && touch.phase != .cancelled
            } ?? Array(changedTouches)

            let orderedChangedTouches = sortedTouches(changedTouches)
            let orderedRemainingTouches = sortedTouches(activeTouches.filter { touch in
                !changedTouchKeys.contains(touchStorageKey(for: touch))
            })

            return orderedChangedTouches + orderedRemainingTouches
        }

        private func touchMessages(from touches: [UITouch]) -> [TouchInterruptMessage] {
            touches.map { touch in
                let location = touch.preciseLocation(in: self)
                let key = touchStorageKey(for: touch)
                let lastPosition = lastTouchPositions[key] ?? location
                let message = TouchInterruptMessage(
                    x: Double(location.x),
                    y: Double(location.y),
                    identifier: touchIdentifier(for: touch),
                    deltaX: Double(location.x - lastPosition.x),
                    deltaY: Double(location.y - lastPosition.y)
                )
                lastTouchPositions[key] = location
                return message
            }
        }

        private func clearTouchPositions(for touches: [UITouch]) {
            for touch in touches {
                lastTouchPositions.removeValue(forKey: touchStorageKey(for: touch))
            }
        }

        private func dispatchTapIfNeeded(endedTouches: [UITouch], event: UIEvent?) {
            let remainingTouches = event?.allTouches?.filter { touch in
                touch.phase != .ended && touch.phase != .cancelled
            } ?? []
            guard remainingTouches.isEmpty,
                  endedTouches.count == 1,
                  let touch = endedTouches.first,
                  touchStorageKey(for: touch) == activeTapTouchKey else {
                return
            }

            let location = touch.preciseLocation(in: self)
            dispatchTap(x: Double(location.x), y: Double(location.y))
        }

        override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
            let activeTouches = orderedActiveTouches(changedTouches: touches, event: event)
            activeTapTouchKey = activeTouches.count == 1 && touches.count == 1
                ? activeTouches.first.map { self.touchStorageKey(for: $0) }
                : nil
            activeTapStartPosition = activeTapTouchKey == nil
                ? nil
                : activeTouches.first?.preciseLocation(in: self)
            dispatchTouchStart(touches: touchMessages(from: activeTouches))
            super.touchesBegan(touches, with: event)
        }

        override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
            if let activeTapTouchKey, let activeTapStartPosition,
               let activeTouch = event?.allTouches?.first(where: {
                   touchStorageKey(for: $0) == activeTapTouchKey
               }) {
                let position = activeTouch.preciseLocation(in: self)
                if hypot(
                    position.x - activeTapStartPosition.x,
                    position.y - activeTapStartPosition.y
                ) > tapMovementTolerance {
                    self.activeTapTouchKey = nil
                    self.activeTapStartPosition = nil
                }
            }
            dispatchTouchMove(touches: touchMessages(from: orderedActiveTouches(changedTouches: touches, event: event)))
            super.touchesMoved(touches, with: event)
        }

        override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
            let orderedTouches = sortedTouches(touches)
            dispatchTouchEnd(touches: touchMessages(from: orderedTouches))
            dispatchTapIfNeeded(endedTouches: orderedTouches, event: event)
            activeTapTouchKey = nil
            activeTapStartPosition = nil
            clearTouchPositions(for: orderedTouches)
            super.touchesEnded(touches, with: event)
        }

        override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
            let orderedTouches = sortedTouches(touches)
            dispatchTouchCancel(touches: touchMessages(from: orderedTouches))
            activeTapTouchKey = nil
            activeTapStartPosition = nil
            clearTouchPositions(for: orderedTouches)
            super.touchesCancelled(touches, with: event)
        }

        private func currentScale() -> CGFloat {
            window?.screen.scale ?? UIScreen.main.scale
        }

        override func layoutSubviews() {
            super.layoutSubviews()

            noteViewportSize(bounds.size)
        }

        private func noteViewportSize(_ size: CGSize) {
            guard size.width > 0, size.height > 0 else {
                return
            }

            if viewportSizesMatch(size, previousViewportSize) {
                return
            }

            previousViewportSize = size
        }

        private func isUsableViewportSize(_ size: CGSize) -> Bool {
            size.width.isFinite
                && size.height.isFinite
                && size.width > 0
                && size.height > 0
        }

        private func viewportSizesMatch(_ lhs: CGSize, _ rhs: CGSize) -> Bool {
            abs(lhs.width - rhs.width) <= viewportSizeEpsilon
                && abs(lhs.height - rhs.height) <= viewportSizeEpsilon
        }

        private func presentedAnimatedSize(for layer: CALayer) -> CGSize? {
            guard let presentationLayer = layer.presentation() else {
                return nil
            }

            let presentedFrameSize = presentationLayer.frame.size
            if isUsableViewportSize(presentedFrameSize)
                && !viewportSizesMatch(presentedFrameSize, layer.frame.size) {
                return presentedFrameSize
            }

            let presentedBoundsSize = presentationLayer.bounds.size
            if isUsableViewportSize(presentedBoundsSize)
                && !viewportSizesMatch(presentedBoundsSize, layer.bounds.size) {
                return presentedBoundsSize
            }

            return nil
        }

        private func overdrawViewportSize(from animatedSize: CGSize, toward targetSize: CGSize) -> CGSize {
            CGSize(
                width: max(animatedSize.width, targetSize.width),
                height: max(animatedSize.height, targetSize.height)
            )
        }

        private func viewportSizeForCurrentFrame() -> CGSize {
            // During UIKit rotation, model geometry can jump to the destination while
            // container presentation layers animate through in-flight bounds or transforms.
            // Use an overdraw viewport while presentation geometry is active so a compositor
            // measurement lag cannot expose the UIKit/window background at the expanding edge.
            var currentLayer: CALayer? = layer
            while let candidate = currentLayer {
                if let animatedSize = presentedAnimatedSize(for: candidate) {
                    return overdrawViewportSize(from: animatedSize, toward: bounds.size)
                }
                currentLayer = candidate.superlayer
            }

            return bounds.size
        }

        private func createDisplayLink() {
            displayLink = CADisplayLink(target: self, selector: #selector(handleDisplayLink))
            if #available(iOS 15.0, *) {
                let maximumFramesPerSecond = UIScreen.main.maximumFramesPerSecond
                displayLink?.preferredFrameRateRange = CAFrameRateRange(
                    minimum: 30,
                    maximum: Float(maximumFramesPerSecond),
                    preferred: Float(maximumFramesPerSecond)
                )
            } else {
                displayLink?.preferredFramesPerSecond = UIScreen.main.maximumFramesPerSecond
            }
            displayLink?.add(to: .current, forMode: .common)
        }

        @objc private func handleDisplayLink(_ displayLink: CADisplayLink) {
            if frameInstrumentationEnabled {
                let requestAnimationFrameStart = CACurrentMediaTime()
                processRequestAnimationFrameQueue()
                let requestAnimationFrameMs = Self.elapsedMilliseconds(since: requestAnimationFrameStart)
                let phases = tick(measurePhases: true)
                frameInstrumentation.record(
                    displayLink: displayLink,
                    requestAnimationFrameMs: requestAnimationFrameMs,
                    phases: phases
                )
            } else {
                processRequestAnimationFrameQueue()
                tick()
            }
        }

        deinit {
            displayLink?.invalidate()
        }

        @discardableResult
        private func tick(measurePhases: Bool = false) -> PaxFramePhaseDurations {
            let totalStart = measurePhases ? CACurrentMediaTime() : 0
            var phaseStart = totalStart
            var phases = PaxFramePhaseDurations()

            let viewportSize = viewportSizeForCurrentFrame()
            if measurePhases {
                phases.viewportMs = Self.elapsedMilliseconds(since: phaseStart)
                phaseStart = CACurrentMediaTime()
            }
            guard viewportSize.width > 0, viewportSize.height > 0 else {
                if measurePhases {
                    phases.totalMs = Self.elapsedMilliseconds(since: totalStart)
                }
                return phases
            }
            noteViewportSize(viewportSize)

            let width = Float(viewportSize.width)
            let height = Float(viewportSize.height)
            let scale = Float(currentScale())

            if PaxEngineContainer.paxEngineContainer == nil {
                PaxEngineContainer.paxEngineContainer = pax_init(width, height)
            }
            if measurePhases {
                phases.engineInitMs = Self.elapsedMilliseconds(since: phaseStart)
                phaseStart = CACurrentMediaTime()
            }

            guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                if measurePhases {
                    phases.totalMs = Self.elapsedMilliseconds(since: totalStart)
                }
                return phases
            }

            // The SwiftUI content intentionally ignores safe areas. Read the owning
            // window's safe rectangle and convert it to the canvas coordinate space.
            // Check before every tick so rotation/window changes reach the same frame's
            // layout, without emitting unchanged geometry or modifying the root bounds.
            if let window {
                let safeRect = convert(window.safeAreaLayoutGuide.layoutFrame, from: window)
                let insets = UIEdgeInsets(
                    top: max(0, safeRect.minY - bounds.minY),
                    left: max(0, safeRect.minX - bounds.minX),
                    bottom: max(0, bounds.maxY - safeRect.maxY),
                    right: max(0, bounds.maxX - safeRect.maxX)
                )
                if publishedSafeAreaInsets != insets {
                    dispatchSafeAreaInsets(
                        top: Double(insets.top), right: Double(insets.right),
                        bottom: Double(insets.bottom), left: Double(insets.left)
                    )
                    publishedSafeAreaInsets = insets
                }
            }

            // Native property updates and all Metal drawables belong to one frame.
            // The Metal backend honors presentsWithTransaction when presenting them.
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            defer { CATransaction.commit() }

            let nativeMessageQueue = pax_tick(
                engineContainer,
                nil,
                width,
                height,
                scale
            )
            if measurePhases {
                phases.paxTickMs = Self.elapsedMilliseconds(since: phaseStart)
                phaseStart = CACurrentMediaTime()
            }
            let queue = nativeMessageQueue.unsafelyUnwrapped.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            processNativeMessageQueueData(Data(buffer: buffer))
            pax_dealloc_message_queue(nativeMessageQueue)
            if measurePhases {
                phases.nativeMessagesMs = Self.elapsedMilliseconds(since: phaseStart)
                phaseStart = CACurrentMediaTime()
            }

            surfaceManager.sync(
                engineContainer: engineContainer,
                rootView: self,
                scale: CGFloat(scale)
            )
            if measurePhases {
                phases.surfaceSyncMs = Self.elapsedMilliseconds(since: phaseStart)
                phaseStart = CACurrentMediaTime()
            }
            pax_render(engineContainer)
            if measurePhases {
                phases.paxRenderMs = Self.elapsedMilliseconds(since: phaseStart)
                phases.totalMs = Self.elapsedMilliseconds(since: totalStart)
            }
            return phases
        }

        private static func frameInstrumentationFlagEnabled() -> Bool {
            if processFlagEnabled("PAX_IOS_FRAME_INSTRUMENTATION") {
                return true
            }
            return UserDefaults.standard.bool(forKey: "PAX_IOS_FRAME_INSTRUMENTATION")
        }

        private static func processFlagEnabled(_ name: String) -> Bool {
            guard let value = ProcessInfo.processInfo.environment[name]?.lowercased() else {
                return false
            }
            return value == "1" || value == "true" || value == "yes" || value == "on"
        }

        private static func elapsedMilliseconds(since start: CFTimeInterval) -> Double {
            (CACurrentMediaTime() - start) * 1000.0
        }

        private struct PaxFramePhaseDurations {
            var viewportMs = 0.0
            var engineInitMs = 0.0
            var paxTickMs = 0.0
            var nativeMessagesMs = 0.0
            var surfaceSyncMs = 0.0
            var paxRenderMs = 0.0
            var totalMs = 0.0
        }

        private struct PaxFrameInstrumentation {
            private var windowStart = CACurrentMediaTime()
            private var lastDisplayLinkTimestamp: CFTimeInterval?
            private var frameCount = 0
            private var callbackIntervalTotalMs = 0.0
            private var callbackIntervalMaxMs = 0.0
            private var overBudgetFrames = 0
            private var frameTotal = PhaseStats()
            private var requestAnimationFrame = PhaseStats()
            private var paxTick = PhaseStats()
            private var nativeMessages = PhaseStats()
            private var surfaceSync = PhaseStats()
            private var paxRender = PhaseStats()

            mutating func record(
                displayLink: CADisplayLink,
                requestAnimationFrameMs: Double,
                phases: PaxFramePhaseDurations
            ) {
                frameCount += 1
                if let lastDisplayLinkTimestamp {
                    let intervalMs = (displayLink.timestamp - lastDisplayLinkTimestamp) * 1000.0
                    callbackIntervalTotalMs += intervalMs
                    callbackIntervalMaxMs = max(callbackIntervalMaxMs, intervalMs)
                }
                lastDisplayLinkTimestamp = displayLink.timestamp

                let budgetMs = max((displayLink.targetTimestamp - displayLink.timestamp) * 1000.0, 1.0)
                if phases.totalMs > budgetMs {
                    overBudgetFrames += 1
                }

                frameTotal.record(phases.totalMs)
                requestAnimationFrame.record(requestAnimationFrameMs)
                paxTick.record(phases.paxTickMs)
                nativeMessages.record(phases.nativeMessagesMs)
                surfaceSync.record(phases.surfaceSyncMs)
                paxRender.record(phases.paxRenderMs)

                let now = CACurrentMediaTime()
                let elapsed = now - windowStart
                guard elapsed >= 1.0 else {
                    return
                }

                let callbackSamples = max(frameCount - 1, 1)
                let callbackAvgMs = callbackIntervalTotalMs / Double(callbackSamples)
                let fps = Double(frameCount) / elapsed
                print(
                    String(
                        format: "[PaxFrame] fps=%.1f frames=%d budget=%.2fms callback_avg=%.2fms callback_max=%.2fms over_budget=%d total=%@ raf=%@ pax_tick=%@ native=%@ surface=%@ render=%@",
                        fps,
                        frameCount,
                        budgetMs,
                        callbackAvgMs,
                        callbackIntervalMaxMs,
                        overBudgetFrames,
                        frameTotal.summary,
                        requestAnimationFrame.summary,
                        paxTick.summary,
                        nativeMessages.summary,
                        surfaceSync.summary,
                        paxRender.summary
                    )
                )
                resetWindow(now: now)
            }

            private mutating func resetWindow(now: CFTimeInterval) {
                windowStart = now
                frameCount = 0
                callbackIntervalTotalMs = 0.0
                callbackIntervalMaxMs = 0.0
                overBudgetFrames = 0
                frameTotal.reset()
                requestAnimationFrame.reset()
                paxTick.reset()
                nativeMessages.reset()
                surfaceSync.reset()
                paxRender.reset()
            }
        }

        private struct PhaseStats {
            private var totalMs = 0.0
            private var maxMs = 0.0
            private var count = 0

            mutating func record(_ value: Double) {
                totalMs += value
                maxMs = max(maxMs, value)
                count += 1
            }

            mutating func reset() {
                totalMs = 0.0
                maxMs = 0.0
                count = 0
            }

            var summary: String {
                guard count > 0 else {
                    return "avg=0.00 max=0.00"
                }
                return String(format: "avg=%.2f max=%.2f", totalMs / Double(count), maxMs)
            }
        }

        private func measureTextElement(_ textElement: TextElement) -> CGSize {
            let label = UILabel()
            label.numberOfLines = 0
            label.text = textElement.content
            label.font = textElement.textStyle.font.getUIFont(size: textElement.textStyle.font_size)

            switch textElement.textStyle.alignmentMultiline {
            case .center:
                label.textAlignment = .center
            case .leading:
                label.textAlignment = .left
            case .trailing:
                label.textAlignment = .right
            @unknown default:
                label.textAlignment = .left
            }

            let constraint = CGSize(
                width: textElement.size_x >= 0 ? CGFloat(textElement.size_x) : CGFloat.greatestFiniteMagnitude,
                height: textElement.size_y >= 0 ? CGFloat(textElement.size_y) : CGFloat.greatestFiniteMagnitude
            )
            let measured = label.sizeThatFits(constraint)
            return CGSize(width: ceil(measured.width), height: ceil(measured.height))
        }

        private func respondToTextMeasurementRequest(_ textElement: TextElement, generation: UInt64) {
            let measuredSize = measureTextElement(textElement)
            textElement.lastMeasuredSize = measuredSize
            recomputeResolvedMask(for: textElement)
            dispatchTextMeasurementResponse(
                id: textElement.id,
                generation: generation,
                width: Double(measuredSize.width),
                height: Double(measuredSize.height)
            )
        }

        func didUpdateTextElement(_ textElement: TextElement, measureGeneration: UInt64?) {
            guard let measureGeneration else {
                return
            }
            respondToTextMeasurementRequest(textElement, generation: measureGeneration)
        }

        func handleNavigate(patch: NavigationPatchMessage) {
            if patch.target == "current", dispatchVirtualRouteNavigation(to: patch.url) {
                return
            }
            guard let url = URL(string: patch.url) else {
                return
            }
            UIApplication.shared.open(url)
        }

        func printAllFilesInBundle() {
            let bundleURL = Bundle.main.bundleURL

            do {
                let resourceURLs = try FileManager.default.contentsOfDirectory(at: bundleURL, includingPropertiesForKeys: nil, options: [])
                for url in resourceURLs {
                    print(url.lastPathComponent)
                }
            } catch {
                print("Error: \(error)")
            }
        }

        func handleImageLoad(patch: ImageLoadPatch) {
            do {
                let fullPatchPath = patch.path!
                let url = URL(fileURLWithPath: fullPatchPath)
                let fileNameWithExtension = url.lastPathComponent
                let fileExtension = url.pathExtension
                let fileName = String(fileNameWithExtension.prefix(fileNameWithExtension.count - fileExtension.count - 1))

                guard let nestedBundleURL = Bundle.main.url(forResource: "PaxSwiftCartridge_PaxCartridgeAssets", withExtension: "bundle") else {
                    throw NSError(domain: "", code: 99, userInfo: [NSLocalizedDescriptionKey : "PaxCartridgeAssets bundle not found in main bundle.  Make sure you have imported PaxCartridgeAssets in Swift."])
                }

                let assetsBundle = Bundle(url: nestedBundleURL)
                
                guard let imageURL = assetsBundle?.url(forResource: fileName, withExtension: fileExtension) else {
                    throw NSError(domain: "", code: 100, userInfo: [NSLocalizedDescriptionKey : "Image file not found in nested bundle"])
                }

                guard let image = UIImage(contentsOfFile: imageURL.path) else {
                    throw NSError(domain: "", code: 101, userInfo: [NSLocalizedDescriptionKey : "Could not create UIImage from data"])
                }

                guard let cgImage = image.cgImage else {
                    throw NSError(domain: "", code: 102, userInfo: [NSLocalizedDescriptionKey : "Could not retrieve CGImage from UIImage"])
                }

                let width = cgImage.width
                let height = cgImage.height
                let bitsPerComponent = cgImage.bitsPerComponent
                let bytesPerRow = cgImage.bytesPerRow
                let totalBytes = height * bytesPerRow

                let colorSpace = CGColorSpaceCreateDeviceRGB()
                let bitmapInfo = CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue

                guard let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: bitsPerComponent, bytesPerRow: bytesPerRow, space: colorSpace, bitmapInfo: bitmapInfo) else {
                    throw NSError(domain: "", code: 103, userInfo: [NSLocalizedDescriptionKey : "Could not create CGContext"])
                }

                context.draw(cgImage, in: CGRect(x: 0, y: 0, width: width, height: height))

                guard let data = context.data else {
                    throw NSError(domain: "", code: 104, userInfo: [NSLocalizedDescriptionKey : "Could not retrieve pixel data from context"])
                }

                let byteBuffer = data.assumingMemoryBound(to: UInt8.self)

                let raw_pointer_uint = UInt(bitPattern: byteBuffer)

                let buffer = try! FlexBufferBuilder.encodeMap { builder in
                    builder.addMapWithStringKey("Image") { imageBuilder in
                        imageBuilder.addMapWithStringKey("Reference") { referenceBuilder in
                            referenceBuilder.addWithStringKey("id", UInt(patch.id))
                            referenceBuilder.addStringWithStringKey("path", fullPatchPath)
                            referenceBuilder.addWithStringKey("image_data", raw_pointer_uint)
                            referenceBuilder.addWithStringKey("image_data_length", UInt(totalBytes))
                            referenceBuilder.addWithStringKey("width", UInt(width))
                            referenceBuilder.addWithStringKey("height", UInt(height))
                        }
                    }
                }

                buffer.data.withUnsafeBytes { ptr in
                    var ffi_container = InterruptBuffer(data_ptr: ptr.baseAddress!, length: UInt64(ptr.count))
                    withUnsafePointer(to: &ffi_container) { ffi_container_ptr in
                        pax_interrupt(PaxEngineContainer.paxEngineContainer!, ffi_container_ptr)
                    }
                }
            } catch {
                print("Failed to load image data: \(error)")
            }
        }
    }
}
