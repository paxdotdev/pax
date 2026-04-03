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
                    }
                }
            }
        }
    } catch {
        print("Error reading font files from resources: \(error)")
    }
}

private func sendInterruptToEngine(data: Data) {
    data.withUnsafeBytes { ptr in
        var ffi_container = InterruptBuffer(data_ptr: ptr.baseAddress!, length: UInt64(ptr.count))
        withUnsafePointer(to: &ffi_container) { ffi_container_ptr in
            pax_interrupt(PaxViewIos.PaxEngineContainer.paxEngineContainer!, ffi_container_ptr)
        }
    }
}

struct PaxViewIos: View {
    init() {
        registerPaxFontsIfNeeded()
        NativeInterruptDispatcher.shared.sendData = sendInterruptToEngine
    }

    func canvasView(size: CGSize) -> some View {
        PaxCanvasViewRepresentable()
            .frame(width: size.width, height: size.height, alignment: .topLeading)
            .gesture(DragGesture(minimumDistance: 0, coordinateSpace: .local)
                .onChanged { dragGesture in
                    if let previous = self.previousScrollLocation {
                        let deltaX = dragGesture.location.x - previous.x
                        let deltaY = dragGesture.location.y - previous.y

                        sendTouchInterrupt(
                            kind: "TouchMove",
                            location: dragGesture.location,
                            delta: CGPoint(x: deltaX, y: deltaY)
                        )

                        let json = String(format: "{\"Scroll\": {\"x\": %f, \"y\": %f, \"delta_x\": %f, \"delta_y\": %f} }",
                                          dragGesture.location.x,
                                          dragGesture.location.y,
                                          -deltaX,
                                          -deltaY)
                        sendInterrupt(with: json)
                    } else {
                        sendTouchInterrupt(
                            kind: "TouchStart",
                            location: dragGesture.location,
                            delta: .zero
                        )
                    }

                    self.previousScrollLocation = dragGesture.location
                }
                .onEnded { dragGesture in
                    let delta: CGPoint
                    if let previous = self.previousScrollLocation {
                        delta = CGPoint(
                            x: dragGesture.location.x - previous.x,
                            y: dragGesture.location.y - previous.y
                        )
                    } else {
                        delta = .zero
                    }
                    sendTouchInterrupt(
                        kind: "TouchEnd",
                        location: dragGesture.location,
                        delta: delta
                    )
                    self.previousScrollLocation = nil

                    let json = String(format: "{\"Click\": {\"x\": %f, \"y\": %f, \"button\": \"Left\", \"modifiers\":[] } }", dragGesture.location.x, dragGesture.location.y)
                    sendInterrupt(with: json)
                }
            )
    }

    @State private var previousScrollLocation: CGPoint? = nil

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
        }
    }

    func sendInterrupt(with json: String) {
        let buffer = try! FlexBufferBuilder.fromJSON(json)
        sendInterrupt(data: buffer.data)
    }

    func sendTouchInterrupt(kind: String, location: CGPoint, delta: CGPoint) {
        let json = String(
            format: "{\"%@\":{\"touches\":[{\"x\":%f,\"y\":%f,\"identifier\":0,\"delta_x\":%f,\"delta_y\":%f}]}}",
            kind,
            location.x,
            location.y,
            delta.x,
            delta.y
        )
        sendInterrupt(with: json)
    }

    func sendInterrupt(data: Data) {
        sendInterruptToEngine(data: data)
    }

    class PaxEngineContainer {
        static var paxEngineContainer : OpaquePointer? = nil
    }

    

    struct PaxCanvasViewRepresentable: UIViewRepresentable {
        typealias UIViewType = PaxCanvasViewIos

        func makeUIView(context: Context) -> PaxCanvasViewIos {
            registerPaxFontsIfNeeded()
            NativeInterruptDispatcher.shared.sendData = sendInterruptToEngine
            let view = PaxCanvasViewIos()
            return view
        }

        func updateUIView(_ uiView: PaxCanvasViewIos, context: Context) {
        }
    }


    class PaxCanvasViewIos: UIView, NativeMessageHandling {

        override class var layerClass: AnyClass {
            CAMetalLayer.self
        }

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
        private var displayLink: CADisplayLink?
        private var previousViewportSize: CGSize = .zero
        private var needsNativeTextRemeasure = false
        private var debugLayoutLogCount = 0
        private var debugTickLogCount = 0
        private var debugHeadingLogCount = 0

        private var metalLayer: CAMetalLayer {
            layer as! CAMetalLayer
        }

        override init(frame: CGRect) {
            super.init(frame: frame)
            configureMetalLayer()
            createDisplayLink()
        }

        required init?(coder: NSCoder) {
            super.init(coder: coder)
            configureMetalLayer()
            createDisplayLink()
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

        private func currentScale() -> CGFloat {
            window?.screen.scale ?? UIScreen.main.scale
        }

        private func configureMetalLayer() {
            isOpaque = false
            let scale = currentScale()
            contentScaleFactor = scale
            metalLayer.contentsScale = scale
            metalLayer.framebufferOnly = false
            metalLayer.isOpaque = false
            metalLayer.presentsWithTransaction = false
            metalLayer.colorspace = CGColorSpace(name: CGColorSpace.sRGB)
        }

        override func layoutSubviews() {
            super.layoutSubviews()
            metalLayer.frame = bounds
            let scale = currentScale()
            contentScaleFactor = scale
            metalLayer.contentsScale = scale
            metalLayer.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
            if debugLayoutLogCount < 8 {
                NSLog("[pax-ios-debug] layoutSubviews bounds=%@ frame=%@ scale=%0.2f",
                      NSCoder.string(for: bounds),
                      NSCoder.string(for: frame),
                      scale)
                debugLayoutLogCount += 1
            }

            if bounds.size != previousViewportSize {
                previousViewportSize = bounds.size
                needsNativeTextRemeasure = true
                for textElement in textElements.elements.values {
                    textElement.lastMeasuredSize = nil
                }
            }
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

        @objc private func handleDisplayLink() {
            processRequestAnimationFrameQueue()
            tick()
        }

        deinit {
            displayLink?.invalidate()
        }

        private func tick() {
            guard bounds.width > 0, bounds.height > 0 else {
                return
            }

            let width = Float(bounds.width)
            let height = Float(bounds.height)
            let scale = Float(currentScale())
            if debugTickLogCount < 8 {
                NSLog("[pax-ios-debug] tick bounds=%@ width=%0.2f height=%0.2f scale=%0.2f engine=%@",
                      NSCoder.string(for: bounds),
                      width,
                      height,
                      scale,
                      PaxEngineContainer.paxEngineContainer == nil ? "nil" : "ready")
                debugTickLogCount += 1
            }

            if PaxEngineContainer.paxEngineContainer == nil {
                PaxEngineContainer.paxEngineContainer = pax_init(width, height)
            }

            guard let engineContainer = PaxEngineContainer.paxEngineContainer else {
                return
            }

            let nativeMessageQueue = pax_tick(
                engineContainer,
                Unmanaged.passUnretained(metalLayer).toOpaque(),
                width,
                height,
                scale
            )
            let queue = nativeMessageQueue.unsafelyUnwrapped.pointee
            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            processNativeMessageQueueData(Data(buffer: buffer))
            pax_dealloc_message_queue(nativeMessageQueue)

            if needsNativeTextRemeasure {
                needsNativeTextRemeasure = false
                for textElement in textElements.elements.values {
                    requestTextResizeIfNeeded(textElement)
                }
            }
        }

        private func sendChassisResizeRequest(id: PaxNodeId, size: CGSize) {
            dispatchChassisResizeRequest(id: id, width: Double(size.width), height: Double(size.height))
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

        func didUpdateTextElement(_ textElement: TextElement) {
            if debugHeadingLogCount < 6, textElement.content == "Afterimage Observatory" {
                NSLog("[pax-ios-debug] heading-update transform=%@ size=(%0.2f,%0.2f) opacity=%0.3f parentFrame=%@ measured=%@",
                      textElement.transform.map { String(format: "%0.3f", $0) }.joined(separator: ","),
                      textElement.size_x,
                      textElement.size_y,
                      textElement.opacity,
                      textElement.parentFrame.map(String.init(describing:)) ?? "nil",
                      textElement.lastMeasuredSize.map { NSCoder.string(for: CGRect(origin: .zero, size: $0)) } ?? "nil")
                debugHeadingLogCount += 1
            }
            requestTextResizeIfNeeded(textElement)
        }

        func handleNavigate(patch: NavigationPatchMessage) {
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
