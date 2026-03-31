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


    class PaxCanvasViewMacos: NSView {

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

        @ObservedObject var textElements = TextElements.singleton
        @ObservedObject var frameElements = FrameElements.singleton
        @ObservedObject var buttonElements = ButtonElements.singleton
        @ObservedObject var checkboxElements = CheckboxElements.singleton
        @ObservedObject var nativeImageElements = NativeImageElements.singleton
        @ObservedObject var youtubeVideoElements = YoutubeVideoElements.singleton
        @ObservedObject var dropdownElements = DropdownElements.singleton
        @ObservedObject var radioSetElements = RadioSetElements.singleton
        @ObservedObject var sliderElements = SliderElements.singleton
        @ObservedObject var textboxElements = TextboxElements.singleton
        @ObservedObject var eventBlockerElements = EventBlockerElements.singleton

        private var displayLink: CVDisplayLink?

        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            self.wantsLayer = true
            self.layer?.drawsAsynchronously = true
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
            CVDisplayLinkCreateWithActiveCGDisplays(&displayLink)
            CVDisplayLinkSetOutputHandler(displayLink!) { [weak self] (_, _, _, _, _) -> CVReturn in
                DispatchQueue.main.async {
                    self?.setNeedsDisplay(self?.bounds ?? NSRect.zero)
                    self?.processRequestAnimationFrameQueue()
                }
                return kCVReturnSuccess
            }
            CVDisplayLinkStart(displayLink!)
        }

        deinit {
            CVDisplayLinkStop(displayLink!)
        }

        override func draw(_ dirtyRect: NSRect) {

            super.draw(dirtyRect)
            guard let context = NSGraphicsContext.current else { return }
            var cgContext = context.cgContext
            let scale = self.window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 1.0
            let width = CFloat(bounds.width)
            let height = CFloat(bounds.height)

            if PaxEngineContainer.paxEngineContainer == nil {
                PaxEngineContainer.paxEngineContainer = pax_init(width, height)
            }

            guard let engineContainer = PaxEngineContainer.paxEngineContainer else { return }

            let nativeMessageQueue = pax_tick(
                engineContainer,
                &cgContext,
                width,
                height,
                CFloat(scale)
            )
            processNativeMessageQueue(queue: nativeMessageQueue.unsafelyUnwrapped.pointee)
            pax_dealloc_message_queue(nativeMessageQueue)

            //This DispatchWorkItem `cancel()` is required because sometimes `draw` will be triggered externally from this loop, which
            //would otherwise create new families of continuously reproducing DispatchWorkItems, each ticking up a frenzy, well past the bounds of our target FPS.
            //This cancellation + shared singleton (`tickWorkItem`) ensures that only one DispatchWorkItem is enqueued at a time.
            if currentTickWorkItem != nil {
                currentTickWorkItem!.cancel()
            }

            currentTickWorkItem = DispatchWorkItem {
                self.setNeedsDisplay(dirtyRect)
                self.displayIfNeeded()
            }

        }

        var currentTickWorkItem : DispatchWorkItem? = nil

        private func resolvedTextMaskSize(_ textElement: TextElement) -> CGSize {
            let width = textElement.size_x >= 0 ? CGFloat(textElement.size_x) : textElement.lastMeasuredSize?.width ?? 0
            let height = textElement.size_y >= 0 ? CGFloat(textElement.size_y) : textElement.lastMeasuredSize?.height ?? 0
            return CGSize(width: max(0, width), height: max(0, height))
        }

        private func recomputeResolvedMask(for textElement: TextElement) {
            let mask = resolveNativeMask(
                elementTransform: textElement.transform,
                parentFrame: textElement.parentFrame,
                patch: textElement.nativeMaskPatch,
                fallbackSize: resolvedTextMaskSize(textElement),
                frames: frameElements.elements
            )
            setResolvedNativeMask(id: textElement.id, mask: mask)
        }

        private func recomputeResolvedMask<T: NativePositionElement>(for element: T) {
            let mask = resolveNativeMask(
                elementTransform: element.transform,
                parentFrame: element.parentFrame,
                patch: element.nativeMaskPatch,
                fallbackSize: CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y))),
                frames: frameElements.elements
            )
            setResolvedNativeMask(id: element.id, mask: mask)
        }

        private func recomputeResolvedMasks<T: NativePositionElement>(in elements: [PaxNodeId: T]) {
            for element in elements.values {
                recomputeResolvedMask(for: element)
            }
        }

        private func recomputeAllResolvedMasks() {
            for textElement in textElements.elements.values {
                recomputeResolvedMask(for: textElement)
            }
            recomputeResolvedMasks(in: buttonElements.elements)
            recomputeResolvedMasks(in: checkboxElements.elements)
            recomputeResolvedMasks(in: nativeImageElements.elements)
            recomputeResolvedMasks(in: youtubeVideoElements.elements)
            recomputeResolvedMasks(in: dropdownElements.elements)
            recomputeResolvedMasks(in: radioSetElements.elements)
            recomputeResolvedMasks(in: sliderElements.elements)
            recomputeResolvedMasks(in: textboxElements.elements)
            recomputeResolvedMasks(in: eventBlockerElements.elements)
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
            if let textElement = textElements.elements[patch.id] {
                recomputeResolvedMask(for: textElement)
            }
            dirty.text = true
        }

        func handleTextUpdate(patch: TextUpdatePatch, dirty: inout DirtyCollections) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyPatch(patch: patch)
                recomputeResolvedMask(for: textElement)
            }
            dirty.text = true
        }

        func handleTextDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            self.textElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.text = true
        }

        func handleFrameCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            frameElements.add(element: FrameElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame))
            recomputeAllResolvedMasks()
            dirty.frame = true
        }

        func handleFrameUpdate(patch: FrameUpdatePatch, dirty: inout DirtyCollections) {
            frameElements.elements[patch.id]?.applyPatch(patch: patch)
            recomputeAllResolvedMasks()
            dirty.frame = true
        }

        func handleFrameDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            frameElements.remove(id: patch.id)
            recomputeAllResolvedMasks()
            dirty.frame = true
        }

        func handleButtonCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            buttonElements.add(element: ButtonElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = buttonElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.button = true
        }

        func handleButtonUpdate(patch: ButtonUpdatePatch, dirty: inout DirtyCollections) {
            if let element = buttonElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.button = true
        }

        func handleButtonDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            buttonElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.button = true
        }

        func handleCheckboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            checkboxElements.add(element: CheckboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = checkboxElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.checkbox = true
        }

        func handleCheckboxUpdate(patch: CheckboxUpdatePatch, dirty: inout DirtyCollections) {
            if let element = checkboxElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.checkbox = true
        }

        func handleCheckboxDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            checkboxElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.checkbox = true
        }

        func handleNativeImageCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            nativeImageElements.add(element: NativeImageElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = nativeImageElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.nativeImage = true
        }

        func handleNativeImageUpdate(patch: NativeImageUpdatePatch, dirty: inout DirtyCollections) {
            if let element = nativeImageElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.nativeImage = true
        }

        func handleNativeImageDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            nativeImageElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.nativeImage = true
        }

        func handleYoutubeVideoCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            youtubeVideoElements.add(element: YoutubeVideoElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = youtubeVideoElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.youtubeVideo = true
        }

        func handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch, dirty: inout DirtyCollections) {
            if let element = youtubeVideoElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.youtubeVideo = true
        }

        func handleYoutubeVideoDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            youtubeVideoElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.youtubeVideo = true
        }

        func handleDropdownCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            dropdownElements.add(element: DropdownElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = dropdownElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.dropdown = true
        }

        func handleDropdownUpdate(patch: DropdownUpdatePatch, dirty: inout DirtyCollections) {
            if let element = dropdownElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.dropdown = true
        }

        func handleDropdownDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            dropdownElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.dropdown = true
        }

        func handleRadioSetCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            radioSetElements.add(element: RadioSetElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = radioSetElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.radioSet = true
        }

        func handleRadioSetUpdate(patch: RadioSetUpdatePatch, dirty: inout DirtyCollections) {
            if let element = radioSetElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.radioSet = true
        }

        func handleRadioSetDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            radioSetElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.radioSet = true
        }

        func handleSliderCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            sliderElements.add(element: SliderElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = sliderElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.slider = true
        }

        func handleSliderUpdate(patch: SliderUpdatePatch, dirty: inout DirtyCollections) {
            if let element = sliderElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.slider = true
        }

        func handleSliderDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            sliderElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.slider = true
        }

        func handleTextboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            textboxElements.add(element: TextboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = textboxElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.textbox = true
        }

        func handleTextboxUpdate(patch: TextboxUpdatePatch, dirty: inout DirtyCollections) {
            if let element = textboxElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.textbox = true
        }

        func handleTextboxDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            textboxElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.textbox = true
        }

        func handleEventBlockerCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
            eventBlockerElements.add(element: EventBlockerElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
            if let element = eventBlockerElements.elements[patch.id] {
                recomputeResolvedMask(for: element)
            }
            dirty.eventBlocker = true
        }

        func handleEventBlockerUpdate(patch: EventBlockerPatchMessage, dirty: inout DirtyCollections) {
            if let element = eventBlockerElements.elements[patch.id] {
                element.applyPatch(patch)
                recomputeResolvedMask(for: element)
            }
            dirty.eventBlocker = true
        }

        func handleEventBlockerDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
            eventBlockerElements.remove(id: patch.id)
            removeResolvedNativeMask(id: patch.id)
            dirty.eventBlocker = true
        }

        func handleNavigate(patch: NavigationPatchMessage) {
            guard let url = URL(string: patch.url) else {
                return
            }
            NSWorkspace.shared.open(url)
        }

        func handleOcclusionUpdate(patch: OcclusionUpdatePatch, dirty: inout DirtyCollections) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: textElement)
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
                recomputeResolvedMask(for: buttonElement)
                dirty.button = true
                return
            }
            if let checkboxElement = checkboxElements.elements[patch.id] {
                checkboxElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: checkboxElement)
                dirty.checkbox = true
                return
            }
            if let nativeImageElement = nativeImageElements.elements[patch.id] {
                nativeImageElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: nativeImageElement)
                dirty.nativeImage = true
                return
            }
            if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
                youtubeVideoElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: youtubeVideoElement)
                dirty.youtubeVideo = true
                return
            }
            if let dropdownElement = dropdownElements.elements[patch.id] {
                dropdownElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: dropdownElement)
                dirty.dropdown = true
                return
            }
            if let radioSetElement = radioSetElements.elements[patch.id] {
                radioSetElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: radioSetElement)
                dirty.radioSet = true
                return
            }
            if let sliderElement = sliderElements.elements[patch.id] {
                sliderElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: sliderElement)
                dirty.slider = true
                return
            }
            if let textboxElement = textboxElements.elements[patch.id] {
                textboxElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: textboxElement)
                dirty.textbox = true
                return
            }
            if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
                eventBlockerElement.applyOcclusionPatch(patch)
                recomputeResolvedMask(for: eventBlockerElement)
                dirty.eventBlocker = true
            }
        }

        func handleNativeMaskUpdate(patch: NativeMaskPatch, dirty: inout DirtyCollections) {
            if let textElement = textElements.elements[patch.id] {
                textElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: textElement)
                dirty.text = true
                return
            }
            if let buttonElement = buttonElements.elements[patch.id] {
                buttonElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: buttonElement)
                dirty.button = true
                return
            }
            if let checkboxElement = checkboxElements.elements[patch.id] {
                checkboxElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: checkboxElement)
                dirty.checkbox = true
                return
            }
            if let nativeImageElement = nativeImageElements.elements[patch.id] {
                nativeImageElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: nativeImageElement)
                dirty.nativeImage = true
                return
            }
            if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
                youtubeVideoElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: youtubeVideoElement)
                dirty.youtubeVideo = true
                return
            }
            if let dropdownElement = dropdownElements.elements[patch.id] {
                dropdownElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: dropdownElement)
                dirty.dropdown = true
                return
            }
            if let radioSetElement = radioSetElements.elements[patch.id] {
                radioSetElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: radioSetElement)
                dirty.radioSet = true
                return
            }
            if let sliderElement = sliderElements.elements[patch.id] {
                sliderElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: sliderElement)
                dirty.slider = true
                return
            }
            if let textboxElement = textboxElements.elements[patch.id] {
                textboxElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: textboxElement)
                dirty.textbox = true
                return
            }
            if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
                eventBlockerElement.applyNativeMaskPatch(patch)
                recomputeResolvedMask(for: eventBlockerElement)
                dirty.eventBlocker = true
            }
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
