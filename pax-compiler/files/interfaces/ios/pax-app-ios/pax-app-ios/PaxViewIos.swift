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

struct PaxViewIos: View {

    var canvasView : some View {
        PaxCanvasViewRepresentable()
            .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
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
        ZStack {
            self.canvasView
            NativeRenderingLayer()
        }
        .onAppear {
            NativeInterruptDispatcher.shared.sendData = { data in
                sendInterrupt(data: data)
            }
            registerFonts()
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
        data.withUnsafeBytes { ptr in
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
        struct DirtyCollections {
            var text = false
            var frame = false
            var button = false
            var checkbox = false
            var nativeImage = false
            var youtubeVideo = false
            var dropdown = false
            var radioSet = false
            var slider = false
            var textbox = false
            var eventBlocker = false

            var hasAny: Bool {
                text || frame || button || checkbox || nativeImage || youtubeVideo || dropdown || radioSet || slider || textbox || eventBlocker
            }
        }

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
        }

        override func layoutSubviews() {
            super.layoutSubviews()
            metalLayer.frame = bounds
            let scale = currentScale()
            contentScaleFactor = scale
            metalLayer.contentsScale = scale
            metalLayer.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)

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
            processNativeMessageQueue(queue: nativeMessageQueue.unsafelyUnwrapped.pointee)
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
            sendChassisResizeRequest(id: textElement.id, size: measuredSize)
        }

        private func publish(_ dirty: DirtyCollections) {
            guard dirty.hasAny else {
                return
            }
            if dirty.text {
                textElements.objectWillChange.send()
            }
            if dirty.frame {
                frameElements.objectWillChange.send()
            }
            if dirty.button {
                buttonElements.objectWillChange.send()
            }
            if dirty.checkbox {
                checkboxElements.objectWillChange.send()
            }
            if dirty.nativeImage {
                nativeImageElements.objectWillChange.send()
            }
            if dirty.youtubeVideo {
                youtubeVideoElements.objectWillChange.send()
            }
            if dirty.dropdown {
                dropdownElements.objectWillChange.send()
            }
            if dirty.radioSet {
                radioSetElements.objectWillChange.send()
            }
            if dirty.slider {
                sliderElements.objectWillChange.send()
            }
            if dirty.textbox {
                textboxElements.objectWillChange.send()
            }
            if dirty.eventBlocker {
                eventBlockerElements.objectWillChange.send()
            }
        }

        func handleTextCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            textElements.add(element: TextElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.text = true
        }

        func handleTextUpdate(patch: TextUpdatePatch, dirty: inout DirtyCollections) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyPatch(patch: patch)
                requestTextResizeIfNeeded(textElement)
            }
            dirty.text = true
        }

        func handleTextDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            self.textElements.remove(id: patch.id)
            dirty.text = true
        }

        func handleFrameCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            frameElements.add(element: FrameElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame))
            dirty.frame = true
        }

        func handleFrameUpdate(patch: FrameUpdatePatch, dirty: inout DirtyCollections) {
            frameElements.elements[patch.id]?.applyPatch(patch: patch)
            dirty.frame = true
        }

        func handleFrameDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            frameElements.remove(id: patch.id)
            dirty.frame = true
        }

        func handleButtonCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            buttonElements.add(element: ButtonElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.button = true
        }

        func handleButtonUpdate(patch: ButtonUpdatePatch, dirty: inout DirtyCollections) {
            buttonElements.elements[patch.id]?.applyPatch(patch)
            dirty.button = true
        }

        func handleButtonDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            buttonElements.remove(id: patch.id)
            dirty.button = true
        }

        func handleCheckboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            checkboxElements.add(element: CheckboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.checkbox = true
        }

        func handleCheckboxUpdate(patch: CheckboxUpdatePatch, dirty: inout DirtyCollections) {
            checkboxElements.elements[patch.id]?.applyPatch(patch)
            dirty.checkbox = true
        }

        func handleCheckboxDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            checkboxElements.remove(id: patch.id)
            dirty.checkbox = true
        }

        func handleNativeImageCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            nativeImageElements.add(element: NativeImageElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.nativeImage = true
        }

        func handleNativeImageUpdate(patch: NativeImageUpdatePatch, dirty: inout DirtyCollections) {
            nativeImageElements.elements[patch.id]?.applyPatch(patch)
            dirty.nativeImage = true
        }

        func handleNativeImageDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            nativeImageElements.remove(id: patch.id)
            dirty.nativeImage = true
        }

        func handleYoutubeVideoCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            youtubeVideoElements.add(element: YoutubeVideoElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.youtubeVideo = true
        }

        func handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch, dirty: inout DirtyCollections) {
            youtubeVideoElements.elements[patch.id]?.applyPatch(patch)
            dirty.youtubeVideo = true
        }

        func handleYoutubeVideoDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            youtubeVideoElements.remove(id: patch.id)
            dirty.youtubeVideo = true
        }

        func handleDropdownCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            dropdownElements.add(element: DropdownElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.dropdown = true
        }

        func handleDropdownUpdate(patch: DropdownUpdatePatch, dirty: inout DirtyCollections) {
            dropdownElements.elements[patch.id]?.applyPatch(patch)
            dirty.dropdown = true
        }

        func handleDropdownDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            dropdownElements.remove(id: patch.id)
            dirty.dropdown = true
        }

        func handleRadioSetCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            radioSetElements.add(element: RadioSetElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.radioSet = true
        }

        func handleRadioSetUpdate(patch: RadioSetUpdatePatch, dirty: inout DirtyCollections) {
            radioSetElements.elements[patch.id]?.applyPatch(patch)
            dirty.radioSet = true
        }

        func handleRadioSetDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            radioSetElements.remove(id: patch.id)
            dirty.radioSet = true
        }

        func handleSliderCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            sliderElements.add(element: SliderElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.slider = true
        }

        func handleSliderUpdate(patch: SliderUpdatePatch, dirty: inout DirtyCollections) {
            sliderElements.elements[patch.id]?.applyPatch(patch)
            dirty.slider = true
        }

        func handleSliderDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            sliderElements.remove(id: patch.id)
            dirty.slider = true
        }

        func handleTextboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            textboxElements.add(element: TextboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.textbox = true
        }

        func handleTextboxUpdate(patch: TextboxUpdatePatch, dirty: inout DirtyCollections) {
            textboxElements.elements[patch.id]?.applyPatch(patch)
            dirty.textbox = true
        }

        func handleTextboxDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            textboxElements.remove(id: patch.id)
            dirty.textbox = true
        }

        func handleEventBlockerCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            eventBlockerElements.add(element: EventBlockerElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dirty.eventBlocker = true
        }

        func handleEventBlockerUpdate(patch: EventBlockerPatchMessage, dirty: inout DirtyCollections) {
            eventBlockerElements.elements[patch.id]?.applyPatch(patch)
            dirty.eventBlocker = true
        }

        func handleEventBlockerDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            eventBlockerElements.remove(id: patch.id)
            dirty.eventBlocker = true
        }

        func handleNavigate(patch: NavigationPatchMessage) {
            guard let url = URL(string: patch.url) else {
                return
            }
            UIApplication.shared.open(url)
        }

        func handleOcclusionUpdate(patch: OcclusionUpdatePatch, dirty: inout DirtyCollections) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyOcclusionPatch(patch)
                dirty.text = true
                return
            }
            if let frameElement = frameElements.elements[patch.id] {
                frameElement.applyOcclusionPatch(patch)
                dirty.frame = true
                return
            }
            if let buttonElement = buttonElements.elements[patch.id] {
                buttonElement.applyOcclusionPatch(patch)
                dirty.button = true
                return
            }
            if let checkboxElement = checkboxElements.elements[patch.id] {
                checkboxElement.applyOcclusionPatch(patch)
                dirty.checkbox = true
                return
            }
            if let nativeImageElement = nativeImageElements.elements[patch.id] {
                nativeImageElement.applyOcclusionPatch(patch)
                dirty.nativeImage = true
                return
            }
            if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
                youtubeVideoElement.applyOcclusionPatch(patch)
                dirty.youtubeVideo = true
                return
            }
            if let dropdownElement = dropdownElements.elements[patch.id] {
                dropdownElement.applyOcclusionPatch(patch)
                dirty.dropdown = true
                return
            }
            if let radioSetElement = radioSetElements.elements[patch.id] {
                radioSetElement.applyOcclusionPatch(patch)
                dirty.radioSet = true
                return
            }
            if let sliderElement = sliderElements.elements[patch.id] {
                sliderElement.applyOcclusionPatch(patch)
                dirty.slider = true
                return
            }
            if let textboxElement = textboxElements.elements[patch.id] {
                textboxElement.applyOcclusionPatch(patch)
                dirty.textbox = true
                return
            }
            if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
                eventBlockerElement.applyOcclusionPatch(patch)
                dirty.eventBlocker = true
            }
        }

        func handleNativeMaskUpdate(patch: NativeMaskPatch, dirty: inout DirtyCollections) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyNativeMaskPatch(patch)
                dirty.text = true
                return
            }
            if let buttonElement = buttonElements.elements[patch.id] {
                buttonElement.applyNativeMaskPatch(patch)
                dirty.button = true
                return
            }
            if let checkboxElement = checkboxElements.elements[patch.id] {
                checkboxElement.applyNativeMaskPatch(patch)
                dirty.checkbox = true
                return
            }
            if let nativeImageElement = nativeImageElements.elements[patch.id] {
                nativeImageElement.applyNativeMaskPatch(patch)
                dirty.nativeImage = true
                return
            }
            if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
                youtubeVideoElement.applyNativeMaskPatch(patch)
                dirty.youtubeVideo = true
                return
            }
            if let dropdownElement = dropdownElements.elements[patch.id] {
                dropdownElement.applyNativeMaskPatch(patch)
                dirty.dropdown = true
                return
            }
            if let radioSetElement = radioSetElements.elements[patch.id] {
                radioSetElement.applyNativeMaskPatch(patch)
                dirty.radioSet = true
                return
            }
            if let sliderElement = sliderElements.elements[patch.id] {
                sliderElement.applyNativeMaskPatch(patch)
                dirty.slider = true
                return
            }
            if let textboxElement = textboxElements.elements[patch.id] {
                textboxElement.applyNativeMaskPatch(patch)
                dirty.textbox = true
                return
            }
            if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
                eventBlockerElement.applyNativeMaskPatch(patch)
                dirty.eventBlocker = true
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



        func processNativeMessageQueue(queue: NativeMessageQueue) {

            let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
            let root = FlexBuffer.decode(data: Data.init(buffer: buffer))!
            var dirty = DirtyCollections()

            root["messages"]?.asVector?.makeIterator().forEach( { message in

                let textCreateMessage = message["TextCreate"]
                if textCreateMessage != nil {
                    handleTextCreate(patch: AnyCreatePatch(fb: textCreateMessage!), dirty: &dirty)
                }

                let textUpdateMessage = message["TextUpdate"]
                if textUpdateMessage != nil {
                    handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage!), dirty: &dirty)
                }

                let textDeleteMessage = message["TextDelete"]
                if textDeleteMessage != nil {
                    handleTextDelete(patch: AnyDeletePatch(fb: textDeleteMessage!), dirty: &dirty)
                }

                let frameCreateMessage = message["FrameCreate"]
                if frameCreateMessage != nil {
                    handleFrameCreate(patch: AnyCreatePatch(fb: frameCreateMessage!), dirty: &dirty)
                }

                let frameUpdateMessage = message["FrameUpdate"]
                if frameUpdateMessage != nil {
                    handleFrameUpdate(patch: FrameUpdatePatch(fb: frameUpdateMessage!), dirty: &dirty)
                }

                let frameDeleteMessage = message["FrameDelete"]
                if frameDeleteMessage != nil {
                    handleFrameDelete(patch: AnyDeletePatch(fb: frameDeleteMessage!), dirty: &dirty)
                }

                let buttonCreateMessage = message["ButtonCreate"]
                if buttonCreateMessage != nil {
                    handleButtonCreate(patch: AnyCreatePatch(fb: buttonCreateMessage!), dirty: &dirty)
                }

                let buttonUpdateMessage = message["ButtonUpdate"]
                if buttonUpdateMessage != nil {
                    handleButtonUpdate(patch: ButtonUpdatePatch(fb: buttonUpdateMessage!), dirty: &dirty)
                }

                let buttonDeleteMessage = message["ButtonDelete"]
                if buttonDeleteMessage != nil {
                    handleButtonDelete(patch: AnyDeletePatch(fb: buttonDeleteMessage!), dirty: &dirty)
                }

                let checkboxCreateMessage = message["CheckboxCreate"]
                if checkboxCreateMessage != nil {
                    handleCheckboxCreate(patch: AnyCreatePatch(fb: checkboxCreateMessage!), dirty: &dirty)
                }

                let checkboxUpdateMessage = message["CheckboxUpdate"]
                if checkboxUpdateMessage != nil {
                    handleCheckboxUpdate(patch: CheckboxUpdatePatch(fb: checkboxUpdateMessage!), dirty: &dirty)
                }

                let checkboxDeleteMessage = message["CheckboxDelete"]
                if checkboxDeleteMessage != nil {
                    handleCheckboxDelete(patch: AnyDeletePatch(fb: checkboxDeleteMessage!), dirty: &dirty)
                }

                let nativeImageCreateMessage = message["NativeImageCreate"]
                if nativeImageCreateMessage != nil {
                    handleNativeImageCreate(patch: AnyCreatePatch(fb: nativeImageCreateMessage!), dirty: &dirty)
                }

                let nativeImageUpdateMessage = message["NativeImageUpdate"]
                if nativeImageUpdateMessage != nil {
                    handleNativeImageUpdate(patch: NativeImageUpdatePatch(fb: nativeImageUpdateMessage!), dirty: &dirty)
                }

                let nativeImageDeleteMessage = message["NativeImageDelete"]
                if nativeImageDeleteMessage != nil {
                    handleNativeImageDelete(patch: AnyDeletePatch(fb: nativeImageDeleteMessage!), dirty: &dirty)
                }

                let youtubeVideoCreateMessage = message["YoutubeVideoCreate"]
                if youtubeVideoCreateMessage != nil {
                    handleYoutubeVideoCreate(patch: AnyCreatePatch(fb: youtubeVideoCreateMessage!), dirty: &dirty)
                }

                let youtubeVideoUpdateMessage = message["YoutubeVideoUpdate"]
                if youtubeVideoUpdateMessage != nil {
                    handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch(fb: youtubeVideoUpdateMessage!), dirty: &dirty)
                }

                let youtubeVideoDeleteMessage = message["YoutubeVideoDelete"]
                if youtubeVideoDeleteMessage != nil {
                    handleYoutubeVideoDelete(patch: AnyDeletePatch(fb: youtubeVideoDeleteMessage!), dirty: &dirty)
                }

                let textboxCreateMessage = message["TextboxCreate"]
                if textboxCreateMessage != nil {
                    handleTextboxCreate(patch: AnyCreatePatch(fb: textboxCreateMessage!), dirty: &dirty)
                }

                let textboxUpdateMessage = message["TextboxUpdate"]
                if textboxUpdateMessage != nil {
                    handleTextboxUpdate(patch: TextboxUpdatePatch(fb: textboxUpdateMessage!), dirty: &dirty)
                }

                let textboxDeleteMessage = message["TextboxDelete"]
                if textboxDeleteMessage != nil {
                    handleTextboxDelete(patch: AnyDeletePatch(fb: textboxDeleteMessage!), dirty: &dirty)
                }

                let sliderCreateMessage = message["SliderCreate"]
                if sliderCreateMessage != nil {
                    handleSliderCreate(patch: AnyCreatePatch(fb: sliderCreateMessage!), dirty: &dirty)
                }

                let sliderUpdateMessage = message["SliderUpdate"]
                if sliderUpdateMessage != nil {
                    handleSliderUpdate(patch: SliderUpdatePatch(fb: sliderUpdateMessage!), dirty: &dirty)
                }

                let sliderDeleteMessage = message["SliderDelete"]
                if sliderDeleteMessage != nil {
                    handleSliderDelete(patch: AnyDeletePatch(fb: sliderDeleteMessage!), dirty: &dirty)
                }

                let dropdownCreateMessage = message["DropdownCreate"]
                if dropdownCreateMessage != nil {
                    handleDropdownCreate(patch: AnyCreatePatch(fb: dropdownCreateMessage!), dirty: &dirty)
                }

                let dropdownUpdateMessage = message["DropdownUpdate"]
                if dropdownUpdateMessage != nil {
                    handleDropdownUpdate(patch: DropdownUpdatePatch(fb: dropdownUpdateMessage!), dirty: &dirty)
                }

                let dropdownDeleteMessage = message["DropdownDelete"]
                if dropdownDeleteMessage != nil {
                    handleDropdownDelete(patch: AnyDeletePatch(fb: dropdownDeleteMessage!), dirty: &dirty)
                }

                let radioSetCreateMessage = message["RadioSetCreate"]
                if radioSetCreateMessage != nil {
                    handleRadioSetCreate(patch: AnyCreatePatch(fb: radioSetCreateMessage!), dirty: &dirty)
                }

                let radioSetUpdateMessage = message["RadioSetUpdate"]
                if radioSetUpdateMessage != nil {
                    handleRadioSetUpdate(patch: RadioSetUpdatePatch(fb: radioSetUpdateMessage!), dirty: &dirty)
                }

                let radioSetDeleteMessage = message["RadioSetDelete"]
                if radioSetDeleteMessage != nil {
                    handleRadioSetDelete(patch: AnyDeletePatch(fb: radioSetDeleteMessage!), dirty: &dirty)
                }

                let eventBlockerCreateMessage = message["EventBlockerCreate"]
                if eventBlockerCreateMessage != nil {
                    handleEventBlockerCreate(patch: AnyCreatePatch(fb: eventBlockerCreateMessage!), dirty: &dirty)
                }

                let eventBlockerUpdateMessage = message["EventBlockerUpdate"]
                if eventBlockerUpdateMessage != nil {
                    handleEventBlockerUpdate(patch: EventBlockerPatchMessage(fb: eventBlockerUpdateMessage!), dirty: &dirty)
                }

                let eventBlockerDeleteMessage = message["EventBlockerDelete"]
                if eventBlockerDeleteMessage != nil {
                    handleEventBlockerDelete(patch: AnyDeletePatch(fb: eventBlockerDeleteMessage!), dirty: &dirty)
                }

                let occlusionUpdateMessage = message["OcclusionUpdate"]
                if occlusionUpdateMessage != nil {
                    handleOcclusionUpdate(patch: OcclusionUpdatePatch(fb: occlusionUpdateMessage!), dirty: &dirty)
                }

                let nativeMaskUpdateMessage = message["NativeMaskUpdate"]
                if nativeMaskUpdateMessage != nil {
                    handleNativeMaskUpdate(patch: NativeMaskPatch(fb: nativeMaskUpdateMessage!), dirty: &dirty)
                }

                let imageLoadMessage = message["ImageLoad"]
                if imageLoadMessage != nil {
                    handleImageLoad(patch: ImageLoadPatch(fb: imageLoadMessage!))
                }

                let navigateMessage = message["Navigate"]
                if navigateMessage != nil {
                    handleNavigate(patch: NavigationPatchMessage(fb: navigateMessage!))
                }

                let _ = message["SetCursor"]
                let _ = message["LayerAdd"]
                let _ = message["ShrinkLayersTo"]
                let _ = message["ScrollerCreate"]
                let _ = message["ScrollerUpdate"]
                let _ = message["ScrollerDelete"]

                //^ Add new message-receive handlers here ^
            })

            publish(dirty)

        }

    }
}
