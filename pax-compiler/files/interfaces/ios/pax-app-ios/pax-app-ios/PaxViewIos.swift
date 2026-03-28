//
//  ContentView.swift
//  interface
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI
import Foundation
import FlexBuffers
import Messages
import Rendering
import PaxCartridgeAssets
import PaxCartridge

struct PaxViewIos: View {

    var canvasView : some View = PaxCanvasViewRepresentable()
            .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
    
    @State private var previousScrollLocation: CGPoint? = nil

    var body: some View {
        ZStack {
            self.canvasView
            NativeRenderingLayer()
        }
        .onAppear {
            registerFonts()
        }
        .gesture(DragGesture(minimumDistance: 0, coordinateSpace: .local)
            .onChanged { dragGesture in
                if let previous = self.previousScrollLocation {
                    let deltaX = dragGesture.location.x - previous.x
                    let deltaY = dragGesture.location.y - previous.y
                    
                    let json = String(format: "{\"Scroll\": {\"x\": %f, \"y\": %f, \"delta_x\": %f, \"delta_y\": %f} }",
                                      dragGesture.location.x,
                                      dragGesture.location.y,
                                      -deltaX,
                                      -deltaY)
                    sendInterrupt(with: json)
                }
                
                self.previousScrollLocation = dragGesture.location
            }
            .onEnded { dragGesture in
                //Reset scroll tracking position
                self.previousScrollLocation = nil
                
                // Handle "Click" events — note that we should probably check to ensure that a maximum distance has not been crossed
                // to rightly handle this as a "click".  Currently this is more of a `touchend`.
                let json = String(format: "{\"Click\": {\"x\": %f, \"y\": %f, \"button\": \"Left\", \"modifiers\":[] } }", dragGesture.location.x, dragGesture.location.y)
                sendInterrupt(with: json)
            }
        )
    }

    func sendInterrupt(with json: String) {
        let buffer = try! FlexBufferBuilder.fromJSON(json)
        buffer.data.withUnsafeBytes { ptr in
            var ffi_container = InterruptBuffer(data_ptr: ptr.baseAddress!, length: UInt64(ptr.count))
            withUnsafePointer(to: &ffi_container) { ffi_container_ptr in
                pax_interrupt(PaxEngineContainer.paxEngineContainer!, ffi_container_ptr)
            }
        }
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

    

    struct PaxCanvasViewRepresentable: UIViewRepresentable {
        typealias UIViewType = PaxCanvasViewIos

        func makeUIView(context: Context) -> PaxCanvasViewIos {
            let view = PaxCanvasViewIos()
            return view
        }

        func updateUIView(_ uiView: PaxCanvasViewIos, context: Context) {
        }
    }


    class PaxCanvasViewIos: UIView {

        @ObservedObject var textElements = TextElements.singleton
        @ObservedObject var frameElements = FrameElements.singleton
        private var displayLink: CADisplayLink?

        override init(frame: CGRect) {
            super.init(frame: frame)
             createDisplayLink()
        }

        required init?(coder: NSCoder) {
            super.init(coder: coder)
             createDisplayLink()
        }


        private var requestAnimationFrameQueue: [() -> Void] = []

        private func processRequestAnimationFrameQueue() {
            // Execute and remove each closure in the array
            while !requestAnimationFrameQueue.isEmpty {
                let closure = requestAnimationFrameQueue.removeFirst()
                closure()
            }
        }

        func requestAnimationFrame(_ closure: @escaping () -> Void) {
            requestAnimationFrameQueue.append(closure)
        }
        

        private func createDisplayLink() {
            displayLink = CADisplayLink(target: self, selector: #selector(handleDisplayLink))
            displayLink?.add(to: .current, forMode: .common)
        }

        @objc private func handleDisplayLink() {
            DispatchQueue.main.async {
                self.setNeedsDisplay()
                self.processRequestAnimationFrameQueue()
            }
        }

        deinit {
            displayLink?.invalidate()
        }
        
        override func draw(_ rect: CGRect) {
            super.draw(rect)
            guard let cgContext = UIGraphicsGetCurrentContext() else { return }
            
            // Apply affine transform to cgContext to emulate macOS's "y-up" coordinate space.
            cgContext.translateBy(x: 0, y: rect.height) // Move the origin to the bottom-left
            cgContext.scaleBy(x: 1.0, y: -1.0) // Reflect over x axis

            if PaxEngineContainer.paxEngineContainer == nil {
                PaxEngineContainer.paxEngineContainer = pax_init()
            } else {
                guard var mutableCGContext = UIGraphicsGetCurrentContext() else { return }
                let nativeMessageQueue = pax_tick(PaxEngineContainer.paxEngineContainer!, &mutableCGContext, Float(rect.width), Float(rect.height))
                processNativeMessageQueue(queue: nativeMessageQueue.unsafelyUnwrapped.pointee)
                pax_dealloc_message_queue(nativeMessageQueue)
            }

            if currentTickWorkItem != nil {
                currentTickWorkItem!.cancel()
            }

            currentTickWorkItem = DispatchWorkItem {
                self.setNeedsDisplay(rect)
                self.setNeedsLayout()
            }

        }
        
        var currentTickWorkItem : DispatchWorkItem? = nil

        private func sendChassisResizeRequest(id: PaxNodeId, size: CGSize) {
            let buffer = try! FlexBufferBuilder.encodeMap { builder in
                builder.addVectorWithStringKey("ChassisResizeRequestCollection") { vectorBuilder in
                    vectorBuilder.addMap { requestBuilder in
                        requestBuilder.addWithStringKey("id", UInt(id))
                        requestBuilder.addWithStringKey("width", Double(size.width))
                        requestBuilder.addWithStringKey("height", Double(size.height))
                    }
                }
            }

            buffer.data.withUnsafeBytes { ptr in
                var ffi_container = InterruptBuffer(data_ptr: ptr.baseAddress!, length: UInt64(ptr.count))
                withUnsafePointer(to: &ffi_container) { ffi_container_ptr in
                    pax_interrupt(PaxEngineContainer.paxEngineContainer!, ffi_container_ptr)
                }
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
            sendChassisResizeRequest(id: textElement.id, size: measuredSize)
        }

        func handleTextCreate(patch: AnyCreatePatch) {
            textElements.add(element: TextElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            textElements.objectWillChange.send()
        }

        func handleTextUpdate(patch: TextUpdatePatch) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyPatch(patch: patch)
                requestTextResizeIfNeeded(textElement)
            }
            textElements.objectWillChange.send()
        }

        func handleTextDelete(patch: AnyDeletePatch) {
            self.textElements.remove(id: patch.id)
            textElements.objectWillChange.send()
        }

        func handleFrameCreate(patch: AnyCreatePatch) {
            frameElements.add(element: FrameElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame))
            frameElements.objectWillChange.send()
        }

        func handleFrameUpdate(patch: FrameUpdatePatch) {
            frameElements.elements[patch.id]?.applyPatch(patch: patch)
            frameElements.objectWillChange.send()
        }

        func handleFrameDelete(patch: AnyDeletePatch) {
            frameElements.remove(id: patch.id)
            frameElements.objectWillChange.send()
        }

        func handleOcclusionUpdate(patch: OcclusionUpdatePatch) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyOcclusionPatch(patch)
                textElements.objectWillChange.send()
                return
            }
            if let frameElement = frameElements.elements[patch.id] {
                frameElement.applyOcclusionPatch(patch)
                frameElements.objectWillChange.send()
            }
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



        func processNativeMessageQueue(queue: NativeMessageQueue) {

            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            let root = FlexBuffer.decode(data: Data.init(buffer: buffer))!

            root["messages"]?.asVector?.makeIterator().forEach( { message in

                let textCreateMessage = message["TextCreate"]
                if textCreateMessage != nil {
                    handleTextCreate(patch: AnyCreatePatch(fb: textCreateMessage!))
                }

                let textUpdateMessage = message["TextUpdate"]
                if textUpdateMessage != nil {
                    handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage!))
                }

                let textDeleteMessage = message["TextDelete"]
                if textDeleteMessage != nil {
                    handleTextDelete(patch: AnyDeletePatch(fb: textDeleteMessage!))
                }

                let frameCreateMessage = message["FrameCreate"]
                if frameCreateMessage != nil {
                    handleFrameCreate(patch: AnyCreatePatch(fb: frameCreateMessage!))
                }

                let frameUpdateMessage = message["FrameUpdate"]
                if frameUpdateMessage != nil {
                    handleFrameUpdate(patch: FrameUpdatePatch(fb: frameUpdateMessage!))
                }

                let frameDeleteMessage = message["FrameDelete"]
                if frameDeleteMessage != nil {
                    handleFrameDelete(patch: AnyDeletePatch(fb: frameDeleteMessage!))
                }

                let occlusionUpdateMessage = message["OcclusionUpdate"]
                if occlusionUpdateMessage != nil {
                    handleOcclusionUpdate(patch: OcclusionUpdatePatch(fb: occlusionUpdateMessage!))
                }

                let imageLoadMessage = message["ImageLoad"]
                if imageLoadMessage != nil {
                    handleImageLoad(patch: ImageLoadPatch(fb: imageLoadMessage!))
                }

                //^ Add new message-receive handlers here ^
            })

        }

    }
}
