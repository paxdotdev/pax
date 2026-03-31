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

                withUnsafePointer(to: &ffi_container) {ffi_container_ptr in
                    pax_interrupt(PaxEngineContainer.paxEngineContainer!, ffi_container_ptr)
                }
            })
        })

    }

    func registerFonts() {

        let nestedBundleURL = Bundle.main.url(forResource: "PaxSwiftCartridge_PaxCartridgeAssets", withExtension: "bundle")!

        let resourceBundle = Bundle(url: nestedBundleURL)!
        
        let resourceURL = resourceBundle.resourceURL!
        
        let fontFileExtensions = ["ttf", "otf"]

        do {
            let resourceFiles = try FileManager.default.contentsOfDirectory(at: resourceURL, includingPropertiesForKeys: nil, options: [])
            for fileURL in resourceFiles {
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
                            }
                        } else {
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
        }

        func handleNavigate(patch: NavigationPatchMessage) {
            guard let url = URL(string: patch.url) else {
                return
            }
            NSWorkspace.shared.open(url)
        }

        func didUpdateTextElement(_ textElement: TextElement) {
            _ = textElement
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
                withUnsafePointer(to: &ffi_container) {ffi_container_ptr in
                    pax_interrupt(PaxEngineContainer.paxEngineContainer!, ffi_container_ptr)
                }
            })
        }

    }
}
