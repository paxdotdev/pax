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

private struct PaxDevLookRequest: Codable {
    let request_id: String
    let kind: String
    let output_dir: String
    let scale: Double
    let period_ms: UInt64
    let duration_ms: UInt64
    let format: String
    let quality: Double?
}

private struct PaxDevRequestEnvelope: Codable {
    let request_id: String
    let kind: String
}

private struct PaxDevCapture: Codable {
    let path: String
    let width: Int
    let height: Int
    let captured_at_ms: UInt64
}

private struct PaxDevLookResponse: Codable {
    let request_id: String
    let status: String
    let captures: [PaxDevCapture]
    let error: String?
}

private struct PaxDevInspectTreeRequest: Codable {
    let request_id: String
    let kind: String
    let max_depth: Int?
}

private struct PaxDevInspectTreePayload: Codable {
    let status: String
    let node_count: Int?
    let tree_json: String?
    let error: String?
}

private struct PaxDevInspectTreeResponse: Codable {
    let request_id: String
    let status: String
    let node_count: Int?
    let tree_json: String?
    let error: String?
}

private struct PaxDevRayCastRequest: Codable {
    let request_id: String
    let kind: String
    let x: Double
    let y: Double
    let hit_invisible: Bool
}

private struct PaxDevRayCastBridgeRequest: Codable {
    let x: Double
    let y: Double
    let hit_invisible: Bool
}

private struct PaxDevNodeListPayload: Codable {
    let status: String
    let node_count: Int?
    let nodes_json: String?
    let error: String?
}

private struct PaxDevRayCastResponse: Codable {
    let request_id: String
    let status: String
    let x: Double
    let y: Double
    let hit_invisible: Bool
    let node_count: Int?
    let nodes_json: String?
    let error: String?
}

private struct PaxDevSelectorQueryRequest: Codable {
    let request_id: String
    let kind: String
    let selector: String
}

private struct PaxDevSelectorQueryBridgeRequest: Codable {
    let selector: String
}

private struct PaxDevSelectorQueryResponse: Codable {
    let request_id: String
    let status: String
    let selector: String
    let node_count: Int?
    let nodes_json: String?
    let error: String?
}

private struct PaxDevReplaceNodeRequest: Codable {
    let request_id: String
    let kind: String
    let component_type_id: String
    let template_node_id: Int
    let subtemplate: String
}

private struct PaxDevReplaceNodeBridgeRequest: Codable {
    let component_type_id: String
    let template_node_id: Int
    let subtemplate: String
}

private struct PaxDevReplaceNodeBridgeResponse: Codable {
    let status: String
    let component_type_id: String
    let template_node_id: Int
    let reload_scope: String
    let reloaded_template_node_id: Int?
    let source_path: String?
    let error: String?
}

private struct PaxDevReplaceNodeResponse: Codable {
    let request_id: String
    let status: String
    let component_type_id: String
    let template_node_id: Int
    let reload_scope: String
    let reloaded_template_node_id: Int?
    let source_path: String?
    let error: String?
}

private struct PaxDevSessionRegistration: Codable {
    let session_id: String
    let platform: String
    let designtime: Bool
    let project_root: String?
    let session_dir: String?
    let app_pid: UInt32?
    let design_server_addr: String?
    let control_kind: String
    let location: String?
    let started_at_ms: UInt64
    var last_seen_ms: UInt64
}

private struct PendingPaxDevLookRequest {
    let request: PaxDevLookRequest
    var captures: [PaxDevCapture]
    var nextCaptureAt: Date
    let deadline: Date
    var captureScheduled: Bool
}

struct PaxViewMacos: View {

    var canvasView : some View = PaxCanvasViewRepresentable()
            .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)

    var body: some View {
        ZStack {
            self.canvasView
            NativeRenderingLayer()
        }
        .onAppear {
            registerFonts()
        }.gesture(DragGesture(minimumDistance: 0, coordinateSpace: .global).onEnded { dragGesture in
                    //FUTURE: especially if parsing is a bottleneck, could use a different encoding than JSON
            let json = String(format: "{\"Click\": {\"x\": %f, \"y\": %f, \"button\": \"Left\", \"modifiers\":[] } }", dragGesture.location.x, dragGesture.location.y);
            let buffer = try! FlexBufferBuilder.fromJSON(json)

            //Send `Click` interrupt
            buffer.data.withUnsafeBytes({ptr in
                var ffi_container = InterruptBuffer( data_ptr: ptr.baseAddress!, length: UInt64(ptr.count) )

                guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                    return
                }

                withUnsafePointer(to: &ffi_container) {ffi_container_ptr in
                    pax_interrupt(engineContainer, ffi_container_ptr)
                }
            })
        })

    }

    func registerFonts() {

        let nestedBundleURL = Bundle.main.url(forResource: "PaxSwiftCartridge_PaxCartridgeAssets", withExtension: "bundle")!

        let resourceBundle = Bundle(url: nestedBundleURL)!
        
        let resourceURL = resourceBundle.resourceURL!
        
        let fontFileExtensions: Set<String> = ["ttf", "otf"]

        do {
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
                            print("Font already registered: \(fontFamily) - PostScript name: \(postscriptName)")
                        }
                    }
                }
            }
        } catch {
            print("Error reading font files from resources: \(error)")
        }
    }


    class PaxEngineContainer {
        static var paxEngineContainer : OpaquePointer? = nil
    }

    

    struct PaxCanvasViewRepresentable: NSViewRepresentable {
        typealias NSViewType = PaxCanvasViewMacos

        func makeNSView(context: Context) -> PaxCanvasViewMacos {
            let view = PaxCanvasViewMacos()
            return view
        }

        func updateNSView(_ canvas: PaxCanvasViewMacos, context: Context) { }
    }


    class PaxCanvasViewMacos: NSView, NativeMessageHandling {

        let textElements = TextElements.singleton
        let frameElements = FrameElements.singleton
        let buttonElements = ButtonElements.singleton
        let checkboxElements = CheckboxElements.singleton
        let nativeImageElements = NativeImageElements.singleton
        let youtubeVideoElements = YoutubeVideoElements.singleton
        let dropdownElements = DropdownElements.singleton
        let radioSetElements = RadioSetElements.singleton
        let sliderElements = SliderElements.singleton
        let textboxElements = TextboxElements.singleton
        let eventBlockerElements = EventBlockerElements.singleton

        private var displayLink: CVDisplayLink?
        private var isShuttingDown = false
        private let tickStateLock = NSLock()
        private var tickScheduled = false

        private var metalLayer: CAMetalLayer {
            layer as! CAMetalLayer
        }

        private let devSessionDir = ProcessInfo.processInfo.environment["PAX_DEV_SESSION_DIR"].map {
            URL(fileURLWithPath: $0, isDirectory: true)
        }
        private let devRegistryFile = ProcessInfo.processInfo.environment["PAX_DEV_REGISTRY_FILE"].map {
            URL(fileURLWithPath: $0, isDirectory: false)
        }
        private var pendingLookRequests: [String: PendingPaxDevLookRequest] = [:]
        private var lastDevPoll: Date = .distantPast
        private var lastDevHeartbeat: Date = .distantPast

        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            self.wantsLayer = true
            configureMetalLayer()
            createDisplayLink()
        }

        required init?(coder: NSCoder) {
            super.init(coder: coder)
            self.wantsLayer = true
            configureMetalLayer()
            createDisplayLink()
        }

        override func makeBackingLayer() -> CALayer {
            CAMetalLayer()
        }

        private var requestAnimationFrameQueue: [() -> Void] = []

        private func processRequestAnimationFrameQueue() {
            let callbacks = requestAnimationFrameQueue
            requestAnimationFrameQueue.removeAll(keepingCapacity: true)
            for closure in callbacks {
                closure()
            }
        }

        func requestAnimationFrame(_ closure: @escaping () -> Void) {
            requestAnimationFrameQueue.append(closure)
        }

        private func createDisplayLink() {
            guard displayLink == nil else {
                return
            }
            CVDisplayLinkCreateWithActiveCGDisplays(&displayLink)
            guard let displayLink else {
                return
            }
            CVDisplayLinkSetOutputHandler(displayLink) { [weak self] (_, _, _, _, _) -> CVReturn in
                guard let self else {
                    return kCVReturnSuccess
                }
                self.tickStateLock.lock()
                let shouldSchedule = !self.isShuttingDown && !self.tickScheduled
                if shouldSchedule {
                    self.tickScheduled = true
                }
                self.tickStateLock.unlock()
                guard shouldSchedule else {
                    return kCVReturnSuccess
                }
                DispatchQueue.main.async {
                    defer {
                        self.tickStateLock.lock()
                        self.tickScheduled = false
                        self.tickStateLock.unlock()
                    }
                    guard !self.isShuttingDown else {
                        return
                    }
                    autoreleasepool {
                        self.processRequestAnimationFrameQueue()
                        self.tick()
                    }
                }
                return kCVReturnSuccess
            }
            CVDisplayLinkStart(displayLink)
        }

        private func stopDisplayLink() {
            guard let displayLink else {
                return
            }
            CVDisplayLinkStop(displayLink)
            self.displayLink = nil
        }

        private func shutdown() {
            guard !isShuttingDown else {
                return
            }
            isShuttingDown = true
            tickStateLock.lock()
            tickScheduled = false
            tickStateLock.unlock()
            stopDisplayLink()
        }

        override func viewWillMove(toWindow newWindow: NSWindow?) {
            if newWindow == nil {
                shutdown()
            }
            super.viewWillMove(toWindow: newWindow)
        }

        deinit {
            shutdown()
        }

        private func currentScale() -> CGFloat {
            window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1.0
        }

        private func configureMetalLayer() {
            wantsLayer = true
            if layer == nil {
                layer = makeBackingLayer()
            }
            metalLayer.framebufferOnly = false
            metalLayer.isOpaque = false
            metalLayer.presentsWithTransaction = false
            metalLayer.contentsScale = currentScale()
            metalLayer.colorspace = CGColorSpace(name: CGColorSpace.sRGB)
            metalLayer.frame = bounds
            metalLayer.drawableSize = CGSize(
                width: bounds.width * metalLayer.contentsScale,
                height: bounds.height * metalLayer.contentsScale
            )
        }

        override func layout() {
            super.layout()
            let scale = currentScale()
            metalLayer.contentsScale = scale
            metalLayer.frame = bounds
            metalLayer.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        }

        private func tick() {
            guard !isShuttingDown else { return }
            guard bounds.width > 0, bounds.height > 0 else { return }

            let scale = currentScale()
            let width = Float(bounds.width)
            let height = Float(bounds.height)

            if PaxEngineContainer.paxEngineContainer == nil {
                PaxEngineContainer.paxEngineContainer = pax_init(width, height)
            }

            guard let engineContainer = PaxEngineContainer.paxEngineContainer else { return }

            let nativeMessageQueue = pax_tick(
                engineContainer,
                Unmanaged.passUnretained(metalLayer).toOpaque(),
                width,
                height,
                Float(scale)
            )
            let queue = nativeMessageQueue.unsafelyUnwrapped.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            processNativeMessageQueueData(Data(buffer: buffer))
            pax_dealloc_message_queue(nativeMessageQueue)
            processDevRequestsIfNeeded()
        }

        func handleNavigate(patch: NavigationPatchMessage) {
            guard let url = URL(string: patch.url) else {
                return
            }
            NSWorkspace.shared.open(url)
        }

        func didUpdateTextElement(_ textElement: TextElement) {
            requestTextResizeIfNeeded(textElement)
        }

        private func sendChassisResizeRequest(id: PaxNodeId, size: CGSize) {
            dispatchChassisResizeRequest(id: id, width: Double(size.width), height: Double(size.height))
        }

        private func measureTextElement(_ textElement: TextElement) -> CGSize {
            let attributed: AttributedString
            if textElement.markdown {
                attributed = (try? AttributedString(
                    markdown: textElement.content,
                    options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
                )) ?? AttributedString(textElement.content)
            } else {
                attributed = AttributedString(textElement.content)
            }
            guard PaxEngineContainer.paxEngineContainer != nil else {
                return .zero
            }

            let nsAttributed = NSMutableAttributedString(attributedString: NSAttributedString(attributed))
            let paragraph = NSMutableParagraphStyle()
            switch textElement.textStyle.alignmentMultiline {
            case .center:
                paragraph.alignment = .center
            case .trailing:
                paragraph.alignment = .right
            default:
                paragraph.alignment = .left
            }

            let fullRange = NSRange(location: 0, length: nsAttributed.length)
            nsAttributed.addAttribute(
                .font,
                value: textElement.textStyle.font.getNSFont(size: textElement.textStyle.font_size),
                range: fullRange
            )
            nsAttributed.addAttribute(
                .foregroundColor,
                value: NSColor(textElement.textStyle.fill),
                range: fullRange
            )
            nsAttributed.addAttribute(.paragraphStyle, value: paragraph, range: fullRange)

            let constraint = CGSize(
                width: textElement.size_x >= 0 ? CGFloat(textElement.size_x) : CGFloat.greatestFiniteMagnitude,
                height: textElement.size_y >= 0 ? CGFloat(textElement.size_y) : CGFloat.greatestFiniteMagnitude
            )
            let measured = nsAttributed.boundingRect(
                with: constraint,
                options: [.usesLineFragmentOrigin, .usesFontLeading]
            ).integral
            return CGSize(width: ceil(measured.width), height: ceil(measured.height))
        }

        private func requestTextResizeIfNeeded(_ textElement: TextElement) {
            guard textElement.size_x < 0 || textElement.size_y < 0 else {
                return
            }

            let measuredSize = measureTextElement(textElement)
            if let priorSize = textElement.lastMeasuredSize,
               abs(priorSize.width - measuredSize.width) < 0.5,
               abs(priorSize.height - measuredSize.height) < 0.5 {
                return
            }

            textElement.lastMeasuredSize = measuredSize
            recomputeResolvedMask(for: textElement)
            sendChassisResizeRequest(id: textElement.id, size: measuredSize)
        }

//        let buffer = try! FlexBufferBuilder.encodeMap { builder in
//            builder.add("id_chain", patch.id_chain)
//            builder.addVector("image_data") { imageBuilder in
//                for byte in imageData {
//                    imageBuilder.add(Int(byte))
//                }
//            }
//        }

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

                guard let image = NSImage(contentsOf: imageURL) else {
                    throw NSError(domain: "", code: 101, userInfo: [NSLocalizedDescriptionKey : "Could not create NSImage from data"])
                }

                guard let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
                    throw NSError(domain: "", code: 102, userInfo: [NSLocalizedDescriptionKey : "Could not create CGImage from NSImage"])
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
        func handleScreenshot(patch: ScreenshotPatch) {
            DispatchQueue.main.async { [weak self] in
                self?.sendScreenshotInterrupt(id: patch.id)
            }
        }

        private func processDevRequestsIfNeeded() {
            guard let devSessionDir = devSessionDir else {
                return
            }

            let now = Date()
            if now.timeIntervalSince(lastDevPoll) >= 0.1 {
                loadPaxDevRequests(from: devSessionDir)
                lastDevPoll = now
            }
            updateDevSessionHeartbeatIfNeeded(now: now)
            scheduleDueLookCaptures()
        }

        private func updateDevSessionHeartbeatIfNeeded(now: Date) {
            guard let devRegistryFile = devRegistryFile else {
                return
            }
            if now.timeIntervalSince(lastDevHeartbeat) < 1.0 {
                return
            }

            do {
                let registryData = try Data(contentsOf: devRegistryFile)
                var session = try JSONDecoder().decode(PaxDevSessionRegistration.self, from: registryData)
                session.last_seen_ms = nowMs()
                let encoder = JSONEncoder()
                encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
                let encoded = try encoder.encode(session)
                try atomicWrite(encoded, to: devRegistryFile)
                lastDevHeartbeat = now
            } catch {
                print("Failed to update dev session heartbeat: \(error)")
            }
        }

        private func loadPaxDevRequests(from sessionDir: URL) {
            let requestsDir = sessionDir.appendingPathComponent("requests", isDirectory: true)
            let responseDir = sessionDir.appendingPathComponent("responses", isDirectory: true)
            do {
                let requestFiles = try FileManager.default.contentsOfDirectory(
                    at: requestsDir,
                    includingPropertiesForKeys: nil,
                    options: [.skipsHiddenFiles]
                )
                for requestFile in requestFiles where requestFile.pathExtension == "json" {
                    let requestId = requestFile.deletingPathExtension().lastPathComponent
                    if pendingLookRequests[requestId] != nil {
                        try? FileManager.default.removeItem(at: requestFile)
                        continue
                    }

                    do {
                        let requestData = try Data(contentsOf: requestFile)
                        let envelope = try JSONDecoder().decode(PaxDevRequestEnvelope.self, from: requestData)
                        switch envelope.kind {
                        case "look":
                            let request = try JSONDecoder().decode(PaxDevLookRequest.self, from: requestData)
                            let duration = TimeInterval(request.duration_ms) / 1000.0
                            pendingLookRequests[request.request_id] = PendingPaxDevLookRequest(
                                request: request,
                                captures: [],
                                nextCaptureAt: Date(),
                                deadline: Date().addingTimeInterval(duration),
                                captureScheduled: false
                            )
                        case "inspect-tree":
                            let request = try JSONDecoder().decode(PaxDevInspectTreeRequest.self, from: requestData)
                            try performInspectTree(request: request, responseDir: responseDir)
                        case "ray-cast":
                            let request = try JSONDecoder().decode(PaxDevRayCastRequest.self, from: requestData)
                            try performRayCast(request: request, responseDir: responseDir)
                        case "selector-query":
                            let request = try JSONDecoder().decode(PaxDevSelectorQueryRequest.self, from: requestData)
                            try performSelectorQuery(request: request, responseDir: responseDir)
                        case "replace-node":
                            let request = try JSONDecoder().decode(PaxDevReplaceNodeRequest.self, from: requestData)
                            try performReplaceNode(request: request, responseDir: responseDir)
                        default:
                            try writePaxDevErrorResponse(
                                requestId: envelope.request_id,
                                error: "unsupported dev request kind: \(envelope.kind)",
                                to: responseDir
                            )
                        }
                        try? FileManager.default.removeItem(at: requestFile)
                    } catch {
                        try writePaxDevErrorResponse(
                            requestId: requestId,
                            error: "failed to decode dev request: \(error.localizedDescription)",
                            to: responseDir
                        )
                        try? FileManager.default.removeItem(at: requestFile)
                    }
                }
            } catch {
                print("Failed to poll dev requests: \(error)")
            }
        }

        private func scheduleDueLookCaptures() {
            let now = Date()
            let requestIds = Array(pendingLookRequests.keys)
            for requestId in requestIds {
                guard var pending = pendingLookRequests[requestId] else {
                    continue
                }
                if pending.captureScheduled || pending.nextCaptureAt > now {
                    continue
                }
                pending.captureScheduled = true
                pendingLookRequests[requestId] = pending
                DispatchQueue.main.async { [weak self] in
                    self?.performLookCapture(requestId: requestId)
                }
            }
        }

        private func performLookCapture(requestId: String) {
            guard var pending = pendingLookRequests[requestId] else {
                return
            }
            pending.captureScheduled = false

            do {
                let capture = try capturePaxDevLookFrame(for: pending.request, captureIndex: pending.captures.count)
                pending.captures.append(capture)
                let now = Date()

                if pending.request.period_ms == 0 || now >= pending.deadline {
                    pendingLookRequests.removeValue(forKey: requestId)
                    try writePaxDevResponse(
                        PaxDevLookResponse(
                            request_id: requestId,
                            status: "ok",
                            captures: pending.captures,
                            error: nil
                        ),
                        requestId: requestId,
                        to: devSessionDir!.appendingPathComponent("responses", isDirectory: true)
                    )
                } else {
                    pending.nextCaptureAt = now.addingTimeInterval(TimeInterval(pending.request.period_ms) / 1000.0)
                    pendingLookRequests[requestId] = pending
                }
            } catch {
                pendingLookRequests.removeValue(forKey: requestId)
                do {
                    try writePaxDevResponse(
                        PaxDevLookResponse(
                            request_id: requestId,
                            status: "error",
                            captures: pending.captures,
                            error: error.localizedDescription
                        ),
                        requestId: requestId,
                        to: devSessionDir!.appendingPathComponent("responses", isDirectory: true)
                    )
                } catch {
                    print("Failed to write dev error response: \(error)")
                }
            }
        }

        private func capturePaxDevLookFrame(for request: PaxDevLookRequest, captureIndex: Int) throws -> PaxDevCapture {
            let bitmap = try captureWindowBitmap(scale: CGFloat(request.scale))
            let outputDir = URL(fileURLWithPath: request.output_dir, isDirectory: true)
            try FileManager.default.createDirectory(
                at: outputDir,
                withIntermediateDirectories: true,
                attributes: nil
            )
            let fileExtension = request.format.lowercased() == "jpeg" ? "jpg" : "png"
            let filename = String(format: "%04d", captureIndex) + "." + fileExtension
            let fileURL = outputDir.appendingPathComponent(filename)
            try writeBitmap(bitmap, to: fileURL, format: request.format.lowercased(), quality: request.quality)
            return PaxDevCapture(
                path: fileURL.path,
                width: bitmap.pixelsWide,
                height: bitmap.pixelsHigh,
                captured_at_ms: nowMs()
            )
        }

        private func performInspectTree(request: PaxDevInspectTreeRequest, responseDir: URL) throws {
            guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                throw NSError(domain: "", code: 208, userInfo: [NSLocalizedDescriptionKey: "Pax engine is not initialized"])
            }

            guard let payloadQueue = pax_designtime_inspect_tree(engineContainer, Int64(request.max_depth ?? -1)) else {
                throw NSError(domain: "", code: 209, userInfo: [NSLocalizedDescriptionKey: "inspect tree returned no payload"])
            }
            defer { pax_dealloc_message_queue(payloadQueue) }

            let queue = payloadQueue.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            let payloadData = Data(buffer: buffer)
            let payload = try JSONDecoder().decode(PaxDevInspectTreePayload.self, from: payloadData)
            try writePaxDevResponse(
                PaxDevInspectTreeResponse(
                    request_id: request.request_id,
                    status: payload.status,
                    node_count: payload.node_count,
                    tree_json: payload.tree_json,
                    error: payload.error
                ),
                requestId: request.request_id,
                to: responseDir
            )
        }

        private func performRayCast(request: PaxDevRayCastRequest, responseDir: URL) throws {
            guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                throw NSError(domain: "", code: 213, userInfo: [NSLocalizedDescriptionKey: "Pax engine is not initialized"])
            }

            let bridgeRequest = PaxDevRayCastBridgeRequest(
                x: request.x,
                y: request.y,
                hit_invisible: request.hit_invisible
            )
            let bridgeRequestData = try JSONEncoder().encode(bridgeRequest)

            let responseQueue: UnsafeMutablePointer<NativeMessageQueue>? = try bridgeRequestData.withUnsafeBytes { rawBuffer in
                guard let baseAddress = rawBuffer.baseAddress else {
                    throw NSError(domain: "", code: 214, userInfo: [NSLocalizedDescriptionKey: "ray-cast request payload was empty"])
                }
                var ffiBuffer = InterruptBuffer(data_ptr: baseAddress, length: UInt64(rawBuffer.count))
                return withUnsafePointer(to: &ffiBuffer) { ffiBufferPtr in
                    pax_designtime_ray_cast(engineContainer, ffiBufferPtr)
                }
            }

            guard let responseQueue else {
                throw NSError(domain: "", code: 215, userInfo: [NSLocalizedDescriptionKey: "ray-cast returned no payload"])
            }
            defer { pax_dealloc_message_queue(responseQueue) }

            let queue = responseQueue.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            let payloadData = Data(buffer: buffer)
            let payload = try JSONDecoder().decode(PaxDevNodeListPayload.self, from: payloadData)
            try writePaxDevResponse(
                PaxDevRayCastResponse(
                    request_id: request.request_id,
                    status: payload.status,
                    x: request.x,
                    y: request.y,
                    hit_invisible: request.hit_invisible,
                    node_count: payload.node_count,
                    nodes_json: payload.nodes_json,
                    error: payload.error
                ),
                requestId: request.request_id,
                to: responseDir
            )
        }

        private func performSelectorQuery(request: PaxDevSelectorQueryRequest, responseDir: URL) throws {
            guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                throw NSError(domain: "", code: 216, userInfo: [NSLocalizedDescriptionKey: "Pax engine is not initialized"])
            }

            let bridgeRequest = PaxDevSelectorQueryBridgeRequest(selector: request.selector)
            let bridgeRequestData = try JSONEncoder().encode(bridgeRequest)

            let responseQueue: UnsafeMutablePointer<NativeMessageQueue>? = try bridgeRequestData.withUnsafeBytes { rawBuffer in
                guard let baseAddress = rawBuffer.baseAddress else {
                    throw NSError(domain: "", code: 217, userInfo: [NSLocalizedDescriptionKey: "selector request payload was empty"])
                }
                var ffiBuffer = InterruptBuffer(data_ptr: baseAddress, length: UInt64(rawBuffer.count))
                return withUnsafePointer(to: &ffiBuffer) { ffiBufferPtr in
                    pax_designtime_selector_query(engineContainer, ffiBufferPtr)
                }
            }

            guard let responseQueue else {
                throw NSError(domain: "", code: 218, userInfo: [NSLocalizedDescriptionKey: "selector returned no payload"])
            }
            defer { pax_dealloc_message_queue(responseQueue) }

            let queue = responseQueue.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            let payloadData = Data(buffer: buffer)
            let payload = try JSONDecoder().decode(PaxDevNodeListPayload.self, from: payloadData)
            try writePaxDevResponse(
                PaxDevSelectorQueryResponse(
                    request_id: request.request_id,
                    status: payload.status,
                    selector: request.selector,
                    node_count: payload.node_count,
                    nodes_json: payload.nodes_json,
                    error: payload.error
                ),
                requestId: request.request_id,
                to: responseDir
            )
        }

        private func performReplaceNode(request: PaxDevReplaceNodeRequest, responseDir: URL) throws {
            guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                throw NSError(domain: "", code: 210, userInfo: [NSLocalizedDescriptionKey: "Pax engine is not initialized"])
            }

            let bridgeRequest = PaxDevReplaceNodeBridgeRequest(
                component_type_id: request.component_type_id,
                template_node_id: request.template_node_id,
                subtemplate: request.subtemplate
            )
            let bridgeRequestData = try JSONEncoder().encode(bridgeRequest)

            let responseQueue: UnsafeMutablePointer<NativeMessageQueue>? = try bridgeRequestData.withUnsafeBytes { rawBuffer in
                guard let baseAddress = rawBuffer.baseAddress else {
                    throw NSError(domain: "", code: 211, userInfo: [NSLocalizedDescriptionKey: "replace-node request payload was empty"])
                }
                var ffiBuffer = InterruptBuffer(data_ptr: baseAddress, length: UInt64(rawBuffer.count))
                return withUnsafePointer(to: &ffiBuffer) { ffiBufferPtr in
                    pax_designtime_replace_node(engineContainer, ffiBufferPtr)
                }
            }

            guard let responseQueue else {
                throw NSError(domain: "", code: 212, userInfo: [NSLocalizedDescriptionKey: "replace-node returned no payload"])
            }
            defer { pax_dealloc_message_queue(responseQueue) }

            let queue = responseQueue.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            let payloadData = Data(buffer: buffer)
            let payload = try JSONDecoder().decode(PaxDevReplaceNodeBridgeResponse.self, from: payloadData)
            try writePaxDevResponse(
                PaxDevReplaceNodeResponse(
                    request_id: request.request_id,
                    status: payload.status,
                    component_type_id: payload.component_type_id,
                    template_node_id: payload.template_node_id,
                    reload_scope: payload.reload_scope,
                    reloaded_template_node_id: payload.reloaded_template_node_id,
                    source_path: payload.source_path,
                    error: payload.error
                ),
                requestId: request.request_id,
                to: responseDir
            )
        }

        private func sendScreenshotInterrupt(id: UInt32) {
            do {
                let bitmap = try captureWindowBitmap(scale: 1.0)
                let (rgbaData, width, height) = try rgbaData(from: bitmap)
                try rgbaData.withUnsafeBytes { dataPtr in
                    guard let baseAddress = dataPtr.baseAddress else {
                        throw NSError(domain: "", code: 200, userInfo: [NSLocalizedDescriptionKey: "Could not access screenshot bytes"])
                    }
                    let rawPointerUInt = UInt(bitPattern: baseAddress)
                    let buffer = try FlexBufferBuilder.encode(
                        ["Screenshot": ["Reference": [
                            "id": Int(id),
                            "path": "",
                            "image_data": rawPointerUInt,
                            "image_data_length": rgbaData.count,
                            "width": width,
                            "height": height,
                        ] as FlxbValueMap] as FlxbValueMap] as FlxbValueMap
                    )
                    buffer.data.withUnsafeBytes { ptr in
                        var ffiContainer = InterruptBuffer(data_ptr: ptr.baseAddress!, length: UInt64(ptr.count))
                        guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                            return
                        }
                        withUnsafePointer(to: &ffiContainer) { ffiContainerPtr in
                            pax_interrupt(engineContainer, ffiContainerPtr)
                        }
                    }
                }
            } catch {
                print("Failed to capture screenshot: \(error)")
            }
        }

        private func captureWindowBitmap(scale: CGFloat) throws -> NSBitmapImageRep {
            guard let rootView = self.window?.contentView else {
                throw NSError(domain: "", code: 201, userInfo: [NSLocalizedDescriptionKey: "Window content view is unavailable"])
            }
            rootView.layoutSubtreeIfNeeded()
            rootView.displayIfNeededIgnoringOpacity()

            let bounds = rootView.bounds
            guard let baseBitmap = rootView.bitmapImageRepForCachingDisplay(in: bounds) else {
                throw NSError(domain: "", code: 202, userInfo: [NSLocalizedDescriptionKey: "Could not allocate bitmap representation"])
            }
            guard let graphicsContext = NSGraphicsContext(bitmapImageRep: baseBitmap) else {
                throw NSError(domain: "", code: 210, userInfo: [NSLocalizedDescriptionKey: "Could not create bitmap graphics context"])
            }

            NSGraphicsContext.saveGraphicsState()
            defer { NSGraphicsContext.restoreGraphicsState() }
            rootView.displayIgnoringOpacity(bounds, in: graphicsContext)

            if abs(scale - 1.0) < 0.0001 {
                return baseBitmap
            }

            let targetWidth = max(Int(CGFloat(baseBitmap.pixelsWide) * scale), 1)
            let targetHeight = max(Int(CGFloat(baseBitmap.pixelsHigh) * scale), 1)
            guard let scaledBitmap = NSBitmapImageRep(
                bitmapDataPlanes: nil,
                pixelsWide: targetWidth,
                pixelsHigh: targetHeight,
                bitsPerSample: 8,
                samplesPerPixel: 4,
                hasAlpha: true,
                isPlanar: false,
                colorSpaceName: .deviceRGB,
                bytesPerRow: targetWidth * 4,
                bitsPerPixel: 32
            ) else {
                throw NSError(domain: "", code: 203, userInfo: [NSLocalizedDescriptionKey: "Could not allocate scaled bitmap"])
            }

            let image = NSImage(size: NSSize(width: CGFloat(baseBitmap.pixelsWide), height: CGFloat(baseBitmap.pixelsHigh)))
            image.addRepresentation(baseBitmap)

            NSGraphicsContext.saveGraphicsState()
            defer { NSGraphicsContext.restoreGraphicsState() }
            guard let graphicsContext = NSGraphicsContext(bitmapImageRep: scaledBitmap) else {
                throw NSError(domain: "", code: 204, userInfo: [NSLocalizedDescriptionKey: "Could not create bitmap graphics context"])
            }
            NSGraphicsContext.current = graphicsContext
            graphicsContext.cgContext.interpolationQuality = .high
            image.draw(
                in: NSRect(x: 0, y: 0, width: CGFloat(targetWidth), height: CGFloat(targetHeight)),
                from: NSRect(x: 0, y: 0, width: CGFloat(baseBitmap.pixelsWide), height: CGFloat(baseBitmap.pixelsHigh)),
                operation: .copy,
                fraction: 1.0
            )
            return scaledBitmap
        }

        private func rgbaData(from bitmap: NSBitmapImageRep) throws -> (Data, Int, Int) {
            guard let cgImage = bitmap.cgImage else {
                throw NSError(domain: "", code: 205, userInfo: [NSLocalizedDescriptionKey: "Could not create CGImage from bitmap"])
            }

            let width = cgImage.width
            let height = cgImage.height
            let bytesPerRow = width * 4
            let colorSpace = CGColorSpaceCreateDeviceRGB()
            let bitmapInfo = CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue

            var rgbaData = Data(count: height * bytesPerRow)
            let rendered = rgbaData.withUnsafeMutableBytes { bytes in
                guard let baseAddress = bytes.baseAddress else {
                    return false
                }
                guard let context = CGContext(
                    data: baseAddress,
                    width: width,
                    height: height,
                    bitsPerComponent: 8,
                    bytesPerRow: bytesPerRow,
                    space: colorSpace,
                    bitmapInfo: bitmapInfo
                ) else {
                    return false
                }
                context.draw(cgImage, in: CGRect(x: 0, y: 0, width: width, height: height))
                return true
            }

            if !rendered {
                throw NSError(domain: "", code: 206, userInfo: [NSLocalizedDescriptionKey: "Could not render RGBA pixel data"])
            }
            return (rgbaData, width, height)
        }

        private func writeBitmap(_ bitmap: NSBitmapImageRep, to url: URL, format: String, quality: Double?) throws {
            let fileType: NSBitmapImageRep.FileType = format == "jpeg" ? .jpeg : .png
            var properties: [NSBitmapImageRep.PropertyKey: Any] = [:]
            if let quality = quality, fileType == .jpeg {
                properties[.compressionFactor] = quality
            }
            guard let data = bitmap.representation(using: fileType, properties: properties) else {
                throw NSError(domain: "", code: 207, userInfo: [NSLocalizedDescriptionKey: "Could not encode screenshot data"])
            }
            try atomicWrite(data, to: url)
        }

        private func writePaxDevResponse<T: Encodable>(_ response: T, requestId: String, to responseDir: URL) throws {
            try FileManager.default.createDirectory(
                at: responseDir,
                withIntermediateDirectories: true,
                attributes: nil
            )
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            let data = try encoder.encode(response)
            try atomicWrite(data, to: responseDir.appendingPathComponent("\(requestId).json"))
        }

        private func writePaxDevErrorResponse(requestId: String, error: String, to responseDir: URL) throws {
            let response = [
                "request_id": requestId,
                "status": "error",
                "captures": [],
                "node_count": NSNull(),
                "tree_json": NSNull(),
                "error": error
            ] as [String : Any]
            let data = try JSONSerialization.data(withJSONObject: response, options: [.prettyPrinted, .sortedKeys])
            try FileManager.default.createDirectory(
                at: responseDir,
                withIntermediateDirectories: true,
                attributes: nil
            )
            try atomicWrite(data, to: responseDir.appendingPathComponent("\(requestId).json"))
        }

        private func atomicWrite(_ data: Data, to url: URL) throws {
            let tempURL = url.deletingPathExtension().appendingPathExtension("tmp")
            try data.write(to: tempURL, options: .atomic)
            if FileManager.default.fileExists(atPath: url.path) {
                try FileManager.default.removeItem(at: url)
            }
            try FileManager.default.moveItem(at: tempURL, to: url)
        }

        private func nowMs() -> UInt64 {
            UInt64(Date().timeIntervalSince1970 * 1000.0)
        }


        override func scrollWheel(with event: NSEvent){
            let deltaX = event.scrollingDeltaX
            let deltaY = -event.scrollingDeltaY
            let x = event.locationInWindow.x;
            let y = event.locationInWindow.y;
            let json = String(format: "{\"Scroll\": {\"x\": %f, \"y\": %f, \"delta_x\": %f, \"delta_y\": %f} }", x, y, deltaX, deltaY);
            let buffer = try! FlexBufferBuilder.fromJSON(json)

            //Send `Scroll` interrupt
            buffer.data.withUnsafeBytes({ptr in
                var ffi_container = InterruptBuffer( data_ptr: ptr.baseAddress!, length: UInt64(ptr.count) )
                guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                    return
                }
                withUnsafePointer(to: &ffi_container) {ffi_container_ptr in
                    pax_interrupt(engineContainer, ffi_container_ptr)
                }
            })
        }

    }
}
