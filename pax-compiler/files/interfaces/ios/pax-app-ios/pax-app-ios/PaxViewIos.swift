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
                let swiftLoggerCallback : @convention(c) (UnsafePointer<CChar>?) -> () = {
                    (msg) -> () in
                    let outputString = String(cString: msg!)
                    print(outputString)
                }

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

        func handleTextCreate(patch: AnyCreatePatch) {
            textElements.add(element: TextElement.makeDefault(id_chain: patch.id_chain, clipping_ids: patch.clipping_ids))
        }

        func handleTextUpdate(patch: TextUpdatePatch) {
            textElements.elements[patch.id_chain]?.applyPatch(patch: patch)
            textElements.objectWillChange.send()
        }

        func handleTextDelete(patch: AnyDeletePatch) {
            self.textElements.remove(id: patch.id_chain)
        }

        func handleFrameCreate(patch: AnyCreatePatch) {
            frameElements.add(element: FrameElement.makeDefault(id_chain: patch.id_chain))
        }

        func handleFrameUpdate(patch: FrameUpdatePatch) {
            frameElements.elements[patch.id_chain]?.applyPatch(patch: patch)
            frameElements.objectWillChange.send()
        }

        func handleFrameDelete(patch: AnyDeletePatch) {
            frameElements.remove(id: patch.id_chain)
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
            Task {
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

                    let id_chain : FlxbValueVector = FlxbValueVector.init(values: patch.id_chain.map { (number) -> FlxbValue in
                        return number as FlxbValue
                    })
                    let raw_pointer_uint = UInt(bitPattern: byteBuffer)

                    let buffer = try! FlexBufferBuilder.encode(
                        [ "Image": [ "Reference": [
                            "id_chain": id_chain,
                            "image_data": raw_pointer_uint,
                            "image_data_length": totalBytes,
                            "width": width,
                            "height": height,
                        ] as FlxbValueMap] as FlxbValueMap ] as FlxbValueMap)

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

                let imageLoadMessage = message["ImageLoad"]
                if imageLoadMessage != nil {
                    handleImageLoad(patch: ImageLoadPatch(fb: imageLoadMessage!))
                }

                //^ Add new message-receive handlers here ^
            })

        }

    }
}
