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

        func handleButtonCreate(patch: AnyCreatePatch) {
            buttonElements.add(element: ButtonElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            buttonElements.objectWillChange.send()
        }

        func handleButtonUpdate(patch: ButtonUpdatePatch) {
            buttonElements.elements[patch.id]?.applyPatch(patch)
            buttonElements.objectWillChange.send()
        }

        func handleButtonDelete(patch: AnyDeletePatch) {
            buttonElements.remove(id: patch.id)
            buttonElements.objectWillChange.send()
        }

        func handleCheckboxCreate(patch: AnyCreatePatch) {
            checkboxElements.add(element: CheckboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            checkboxElements.objectWillChange.send()
        }

        func handleCheckboxUpdate(patch: CheckboxUpdatePatch) {
            checkboxElements.elements[patch.id]?.applyPatch(patch)
            checkboxElements.objectWillChange.send()
        }

        func handleCheckboxDelete(patch: AnyDeletePatch) {
            checkboxElements.remove(id: patch.id)
            checkboxElements.objectWillChange.send()
        }

        func handleNativeImageCreate(patch: AnyCreatePatch) {
            nativeImageElements.add(element: NativeImageElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            nativeImageElements.objectWillChange.send()
        }

        func handleNativeImageUpdate(patch: NativeImageUpdatePatch) {
            nativeImageElements.elements[patch.id]?.applyPatch(patch)
            nativeImageElements.objectWillChange.send()
        }

        func handleNativeImageDelete(patch: AnyDeletePatch) {
            nativeImageElements.remove(id: patch.id)
            nativeImageElements.objectWillChange.send()
        }

        func handleYoutubeVideoCreate(patch: AnyCreatePatch) {
            youtubeVideoElements.add(element: YoutubeVideoElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            youtubeVideoElements.objectWillChange.send()
        }

        func handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch) {
            youtubeVideoElements.elements[patch.id]?.applyPatch(patch)
            youtubeVideoElements.objectWillChange.send()
        }

        func handleYoutubeVideoDelete(patch: AnyDeletePatch) {
            youtubeVideoElements.remove(id: patch.id)
            youtubeVideoElements.objectWillChange.send()
        }

        func handleDropdownCreate(patch: AnyCreatePatch) {
            dropdownElements.add(element: DropdownElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            dropdownElements.objectWillChange.send()
        }

        func handleDropdownUpdate(patch: DropdownUpdatePatch) {
            dropdownElements.elements[patch.id]?.applyPatch(patch)
            dropdownElements.objectWillChange.send()
        }

        func handleDropdownDelete(patch: AnyDeletePatch) {
            dropdownElements.remove(id: patch.id)
            dropdownElements.objectWillChange.send()
        }

        func handleRadioSetCreate(patch: AnyCreatePatch) {
            radioSetElements.add(element: RadioSetElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            radioSetElements.objectWillChange.send()
        }

        func handleRadioSetUpdate(patch: RadioSetUpdatePatch) {
            radioSetElements.elements[patch.id]?.applyPatch(patch)
            radioSetElements.objectWillChange.send()
        }

        func handleRadioSetDelete(patch: AnyDeletePatch) {
            radioSetElements.remove(id: patch.id)
            radioSetElements.objectWillChange.send()
        }

        func handleSliderCreate(patch: AnyCreatePatch) {
            sliderElements.add(element: SliderElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            sliderElements.objectWillChange.send()
        }

        func handleSliderUpdate(patch: SliderUpdatePatch) {
            sliderElements.elements[patch.id]?.applyPatch(patch)
            sliderElements.objectWillChange.send()
        }

        func handleSliderDelete(patch: AnyDeletePatch) {
            sliderElements.remove(id: patch.id)
            sliderElements.objectWillChange.send()
        }

        func handleTextboxCreate(patch: AnyCreatePatch) {
            textboxElements.add(element: TextboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            textboxElements.objectWillChange.send()
        }

        func handleTextboxUpdate(patch: TextboxUpdatePatch) {
            textboxElements.elements[patch.id]?.applyPatch(patch)
            textboxElements.objectWillChange.send()
        }

        func handleTextboxDelete(patch: AnyDeletePatch) {
            textboxElements.remove(id: patch.id)
            textboxElements.objectWillChange.send()
        }

        func handleEventBlockerCreate(patch: AnyCreatePatch) {
            eventBlockerElements.add(element: EventBlockerElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            eventBlockerElements.objectWillChange.send()
        }

        func handleEventBlockerUpdate(patch: EventBlockerPatchMessage) {
            eventBlockerElements.elements[patch.id]?.applyPatch(patch)
            eventBlockerElements.objectWillChange.send()
        }

        func handleEventBlockerDelete(patch: AnyDeletePatch) {
            eventBlockerElements.remove(id: patch.id)
            eventBlockerElements.objectWillChange.send()
        }

        func handleNavigate(patch: NavigationPatchMessage) {
            guard let url = URL(string: patch.url) else {
                return
            }
            UIApplication.shared.open(url)
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
                return
            }
            if let buttonElement = buttonElements.elements[patch.id] {
                buttonElement.applyOcclusionPatch(patch)
                buttonElements.objectWillChange.send()
                return
            }
            if let checkboxElement = checkboxElements.elements[patch.id] {
                checkboxElement.applyOcclusionPatch(patch)
                checkboxElements.objectWillChange.send()
                return
            }
            if let nativeImageElement = nativeImageElements.elements[patch.id] {
                nativeImageElement.applyOcclusionPatch(patch)
                nativeImageElements.objectWillChange.send()
                return
            }
            if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
                youtubeVideoElement.applyOcclusionPatch(patch)
                youtubeVideoElements.objectWillChange.send()
                return
            }
            if let dropdownElement = dropdownElements.elements[patch.id] {
                dropdownElement.applyOcclusionPatch(patch)
                dropdownElements.objectWillChange.send()
                return
            }
            if let radioSetElement = radioSetElements.elements[patch.id] {
                radioSetElement.applyOcclusionPatch(patch)
                radioSetElements.objectWillChange.send()
                return
            }
            if let sliderElement = sliderElements.elements[patch.id] {
                sliderElement.applyOcclusionPatch(patch)
                sliderElements.objectWillChange.send()
                return
            }
            if let textboxElement = textboxElements.elements[patch.id] {
                textboxElement.applyOcclusionPatch(patch)
                textboxElements.objectWillChange.send()
                return
            }
            if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
                eventBlockerElement.applyOcclusionPatch(patch)
                eventBlockerElements.objectWillChange.send()
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

                let buttonCreateMessage = message["ButtonCreate"]
                if buttonCreateMessage != nil {
                    handleButtonCreate(patch: AnyCreatePatch(fb: buttonCreateMessage!))
                }

                let buttonUpdateMessage = message["ButtonUpdate"]
                if buttonUpdateMessage != nil {
                    handleButtonUpdate(patch: ButtonUpdatePatch(fb: buttonUpdateMessage!))
                }

                let buttonDeleteMessage = message["ButtonDelete"]
                if buttonDeleteMessage != nil {
                    handleButtonDelete(patch: AnyDeletePatch(fb: buttonDeleteMessage!))
                }

                let checkboxCreateMessage = message["CheckboxCreate"]
                if checkboxCreateMessage != nil {
                    handleCheckboxCreate(patch: AnyCreatePatch(fb: checkboxCreateMessage!))
                }

                let checkboxUpdateMessage = message["CheckboxUpdate"]
                if checkboxUpdateMessage != nil {
                    handleCheckboxUpdate(patch: CheckboxUpdatePatch(fb: checkboxUpdateMessage!))
                }

                let checkboxDeleteMessage = message["CheckboxDelete"]
                if checkboxDeleteMessage != nil {
                    handleCheckboxDelete(patch: AnyDeletePatch(fb: checkboxDeleteMessage!))
                }

                let nativeImageCreateMessage = message["NativeImageCreate"]
                if nativeImageCreateMessage != nil {
                    handleNativeImageCreate(patch: AnyCreatePatch(fb: nativeImageCreateMessage!))
                }

                let nativeImageUpdateMessage = message["NativeImageUpdate"]
                if nativeImageUpdateMessage != nil {
                    handleNativeImageUpdate(patch: NativeImageUpdatePatch(fb: nativeImageUpdateMessage!))
                }

                let nativeImageDeleteMessage = message["NativeImageDelete"]
                if nativeImageDeleteMessage != nil {
                    handleNativeImageDelete(patch: AnyDeletePatch(fb: nativeImageDeleteMessage!))
                }

                let youtubeVideoCreateMessage = message["YoutubeVideoCreate"]
                if youtubeVideoCreateMessage != nil {
                    handleYoutubeVideoCreate(patch: AnyCreatePatch(fb: youtubeVideoCreateMessage!))
                }

                let youtubeVideoUpdateMessage = message["YoutubeVideoUpdate"]
                if youtubeVideoUpdateMessage != nil {
                    handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch(fb: youtubeVideoUpdateMessage!))
                }

                let youtubeVideoDeleteMessage = message["YoutubeVideoDelete"]
                if youtubeVideoDeleteMessage != nil {
                    handleYoutubeVideoDelete(patch: AnyDeletePatch(fb: youtubeVideoDeleteMessage!))
                }

                let textboxCreateMessage = message["TextboxCreate"]
                if textboxCreateMessage != nil {
                    handleTextboxCreate(patch: AnyCreatePatch(fb: textboxCreateMessage!))
                }

                let textboxUpdateMessage = message["TextboxUpdate"]
                if textboxUpdateMessage != nil {
                    handleTextboxUpdate(patch: TextboxUpdatePatch(fb: textboxUpdateMessage!))
                }

                let textboxDeleteMessage = message["TextboxDelete"]
                if textboxDeleteMessage != nil {
                    handleTextboxDelete(patch: AnyDeletePatch(fb: textboxDeleteMessage!))
                }

                let sliderCreateMessage = message["SliderCreate"]
                if sliderCreateMessage != nil {
                    handleSliderCreate(patch: AnyCreatePatch(fb: sliderCreateMessage!))
                }

                let sliderUpdateMessage = message["SliderUpdate"]
                if sliderUpdateMessage != nil {
                    handleSliderUpdate(patch: SliderUpdatePatch(fb: sliderUpdateMessage!))
                }

                let sliderDeleteMessage = message["SliderDelete"]
                if sliderDeleteMessage != nil {
                    handleSliderDelete(patch: AnyDeletePatch(fb: sliderDeleteMessage!))
                }

                let dropdownCreateMessage = message["DropdownCreate"]
                if dropdownCreateMessage != nil {
                    handleDropdownCreate(patch: AnyCreatePatch(fb: dropdownCreateMessage!))
                }

                let dropdownUpdateMessage = message["DropdownUpdate"]
                if dropdownUpdateMessage != nil {
                    handleDropdownUpdate(patch: DropdownUpdatePatch(fb: dropdownUpdateMessage!))
                }

                let dropdownDeleteMessage = message["DropdownDelete"]
                if dropdownDeleteMessage != nil {
                    handleDropdownDelete(patch: AnyDeletePatch(fb: dropdownDeleteMessage!))
                }

                let radioSetCreateMessage = message["RadioSetCreate"]
                if radioSetCreateMessage != nil {
                    handleRadioSetCreate(patch: AnyCreatePatch(fb: radioSetCreateMessage!))
                }

                let radioSetUpdateMessage = message["RadioSetUpdate"]
                if radioSetUpdateMessage != nil {
                    handleRadioSetUpdate(patch: RadioSetUpdatePatch(fb: radioSetUpdateMessage!))
                }

                let radioSetDeleteMessage = message["RadioSetDelete"]
                if radioSetDeleteMessage != nil {
                    handleRadioSetDelete(patch: AnyDeletePatch(fb: radioSetDeleteMessage!))
                }

                let eventBlockerCreateMessage = message["EventBlockerCreate"]
                if eventBlockerCreateMessage != nil {
                    handleEventBlockerCreate(patch: AnyCreatePatch(fb: eventBlockerCreateMessage!))
                }

                let eventBlockerUpdateMessage = message["EventBlockerUpdate"]
                if eventBlockerUpdateMessage != nil {
                    handleEventBlockerUpdate(patch: EventBlockerPatchMessage(fb: eventBlockerUpdateMessage!))
                }

                let eventBlockerDeleteMessage = message["EventBlockerDelete"]
                if eventBlockerDeleteMessage != nil {
                    handleEventBlockerDelete(patch: AnyDeletePatch(fb: eventBlockerDeleteMessage!))
                }

                let occlusionUpdateMessage = message["OcclusionUpdate"]
                if occlusionUpdateMessage != nil {
                    handleOcclusionUpdate(patch: OcclusionUpdatePatch(fb: occlusionUpdateMessage!))
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

        }

    }
}
