import Foundation
import SwiftUI
import FlexBuffers
import Messages

public struct DirtyCollections {
    public var text = false
    public var frame = false
    public var button = false
    public var checkbox = false
    public var nativeImage = false
    public var youtubeVideo = false
    public var dropdown = false
    public var radioSet = false
    public var slider = false
    public var textbox = false
    public var eventBlocker = false

    public var hasAny: Bool {
        text || frame || button || checkbox || nativeImage || youtubeVideo || dropdown || radioSet || slider || textbox || eventBlocker
    }

    public init() {}
}

public struct DirtyResolvedMasks {
    public var elementIds: Set<PaxNodeId> = []
    public var recomputeAll = false

    public init() {}

    public mutating func mark(_ id: PaxNodeId) {
        elementIds.insert(id)
    }
}

public protocol NativeMessageHandling: AnyObject {
    var textElements: TextElements { get }
    var frameElements: FrameElements { get }
    var buttonElements: ButtonElements { get }
    var checkboxElements: CheckboxElements { get }
    var nativeImageElements: NativeImageElements { get }
    var youtubeVideoElements: YoutubeVideoElements { get }
    var dropdownElements: DropdownElements { get }
    var radioSetElements: RadioSetElements { get }
    var sliderElements: SliderElements { get }
    var textboxElements: TextboxElements { get }
    var eventBlockerElements: EventBlockerElements { get }

    func handleImageLoad(patch: ImageLoadPatch)
    func handleNavigate(patch: NavigationPatchMessage)
    func didUpdateTextElement(_ textElement: TextElement)
}

public extension NativeMessageHandling {
    private func geometryChanged<T: NativePositionElement>(
        _ element: T,
        previousTransform: [Float],
        previousSizeX: Float,
        previousSizeY: Float
    ) -> Bool {
        previousTransform != element.transform
            || previousSizeX != element.size_x
            || previousSizeY != element.size_y
    }

    private func textGeometryChanged(
        _ element: TextElement,
        previousTransform: [Float],
        previousSizeX: Float,
        previousSizeY: Float
    ) -> Bool {
        previousTransform != element.transform
            || previousSizeX != element.size_x
            || previousSizeY != element.size_y
    }

    private func frameUpdateAffectsMasks(_ frame: FrameElement, patch: FrameUpdatePatch) -> Bool {
        let wasClipping = frame.clipContent
        let willClip = patch.clipContent ?? frame.clipContent
        guard wasClipping || willClip else {
            return false
        }

        let nextClipPath = patch.clipPath.map { $0.isEmpty ? nil : $0 } ?? frame.clipPath
        let transformChanged = patch.transform != nil && patch.transform! != frame.transform
        let sizeChanged = (patch.size_x != nil && patch.size_x! != frame.size_x)
            || (patch.size_y != nil && patch.size_y! != frame.size_y)
        let clipContentChanged = patch.clipContent != nil && patch.clipContent! != frame.clipContent
        let clipPathChanged = patch.clipPath != nil && nextClipPath != frame.clipPath

        return transformChanged || sizeChanged || clipContentChanged || clipPathChanged
    }

    private func frameOcclusionAffectsMasks(_ frame: FrameElement, patch: OcclusionUpdatePatch) -> Bool {
        frame.clipContent && frame.parentFrame != patch.parentFrame
    }

    private func recomputeResolvedMaskIfPresent(id: PaxNodeId) {
        if let textElement = textElements.elements[id] {
            recomputeResolvedMask(for: textElement)
            return
        }
        if let element = buttonElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = checkboxElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = nativeImageElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = youtubeVideoElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = dropdownElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = radioSetElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = sliderElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = textboxElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = eventBlockerElements.elements[id] {
            recomputeResolvedMask(for: element)
        }
    }

    private func recomputeResolvedMasks(for ids: Set<PaxNodeId>) {
        for id in ids {
            recomputeResolvedMaskIfPresent(id: id)
        }
    }

    func didUpdateTextElement(_ textElement: TextElement) {
        _ = textElement
    }

    func resolvedTextMaskSize(_ textElement: TextElement) -> CGSize {
        let width = textElement.size_x >= 0 ? CGFloat(textElement.size_x) : textElement.lastMeasuredSize?.width ?? 0
        let height = textElement.size_y >= 0 ? CGFloat(textElement.size_y) : textElement.lastMeasuredSize?.height ?? 0
        return CGSize(width: max(0, width), height: max(0, height))
    }

    func recomputeResolvedMask(for textElement: TextElement) {
        let mask = resolveNativeMask(
            elementTransform: textElement.transform,
            parentFrame: textElement.parentFrame,
            patch: textElement.nativeMaskPatch,
            fallbackSize: resolvedTextMaskSize(textElement),
            frames: frameElements.elements
        )
        setResolvedNativeMask(id: textElement.id, mask: mask)
    }

    func recomputeResolvedMask<T: NativePositionElement>(for element: T) {
        let mask = resolveNativeMask(
            elementTransform: element.transform,
            parentFrame: element.parentFrame,
            patch: element.nativeMaskPatch,
            fallbackSize: CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y))),
            frames: frameElements.elements
        )
        setResolvedNativeMask(id: element.id, mask: mask)
    }

    func recomputeResolvedMasks<T: NativePositionElement>(in elements: [PaxNodeId: T]) {
        for element in elements.values {
            recomputeResolvedMask(for: element)
        }
    }

    func recomputeAllResolvedMasks() {
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

    func publish(_ dirty: DirtyCollections) {
        guard dirty.hasAny else {
            return
        }
        NativeSceneInvalidation.singleton.invalidate()
    }

    func handleTextCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        textElements.add(element: TextElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.text = true
    }

    func handleTextUpdate(patch: TextUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let textElement = textElements.elements[patch.id] {
            let previousTransform = textElement.transform
            let previousSizeX = textElement.size_x
            let previousSizeY = textElement.size_y
            textElement.applyPatch(patch: patch)
            if textGeometryChanged(
                textElement,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
            didUpdateTextElement(textElement)
        }
        dirty.text = true
    }

    func handleTextDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        textElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.text = true
    }

    func handleFrameCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections) {
        frameElements.add(element: FrameElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame))
        dirty.frame = true
    }

    func handleFrameUpdate(patch: FrameUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let frame = frameElements.elements[patch.id] {
            if frameUpdateAffectsMasks(frame, patch: patch) {
                masks.recomputeAll = true
            }
            frame.applyPatch(patch: patch)
        }
        dirty.frame = true
    }

    func handleFrameDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let frame = frameElements.elements[patch.id], frame.clipContent {
            masks.recomputeAll = true
        }
        frameElements.remove(id: patch.id)
        dirty.frame = true
    }

    func handleButtonCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        buttonElements.add(element: ButtonElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.button = true
    }

    func handleButtonUpdate(patch: ButtonUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = buttonElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.button = true
    }

    func handleButtonDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        buttonElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.button = true
    }

    func handleCheckboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        checkboxElements.add(element: CheckboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.checkbox = true
    }

    func handleCheckboxUpdate(patch: CheckboxUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = checkboxElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.checkbox = true
    }

    func handleCheckboxDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        checkboxElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.checkbox = true
    }

    func handleNativeImageCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        nativeImageElements.add(element: NativeImageElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.nativeImage = true
    }

    func handleNativeImageUpdate(patch: NativeImageUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = nativeImageElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.nativeImage = true
    }

    func handleNativeImageDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        nativeImageElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.nativeImage = true
    }

    func handleYoutubeVideoCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        youtubeVideoElements.add(element: YoutubeVideoElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.youtubeVideo = true
    }

    func handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = youtubeVideoElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.youtubeVideo = true
    }

    func handleYoutubeVideoDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        youtubeVideoElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.youtubeVideo = true
    }

    func handleDropdownCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        dropdownElements.add(element: DropdownElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.dropdown = true
    }

    func handleDropdownUpdate(patch: DropdownUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = dropdownElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.dropdown = true
    }

    func handleDropdownDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        dropdownElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.dropdown = true
    }

    func handleRadioSetCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        radioSetElements.add(element: RadioSetElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.radioSet = true
    }

    func handleRadioSetUpdate(patch: RadioSetUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = radioSetElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.radioSet = true
    }

    func handleRadioSetDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        radioSetElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.radioSet = true
    }

    func handleSliderCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        sliderElements.add(element: SliderElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.slider = true
    }

    func handleSliderUpdate(patch: SliderUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = sliderElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.slider = true
    }

    func handleSliderDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        sliderElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.slider = true
    }

    func handleTextboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        textboxElements.add(element: TextboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.textbox = true
    }

    func handleTextboxUpdate(patch: TextboxUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = textboxElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.textbox = true
    }

    func handleTextboxDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        textboxElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.textbox = true
    }

    func handleEventBlockerCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        eventBlockerElements.add(element: EventBlockerElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, occlusionLayerId: patch.occlusionLayerId))
        masks.mark(patch.id)
        dirty.eventBlocker = true
    }

    func handleEventBlockerUpdate(patch: EventBlockerPatchMessage, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = eventBlockerElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.eventBlocker = true
    }

    func handleEventBlockerDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        eventBlockerElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.eventBlocker = true
    }

    func handleOcclusionUpdate(patch: OcclusionUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let textElement = textElements.elements[patch.id] {
            let previousParentFrame = textElement.parentFrame
            textElement.applyOcclusionPatch(patch)
            if previousParentFrame != textElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.text = true
            return
        }
        if let frameElement = frameElements.elements[patch.id] {
            if frameOcclusionAffectsMasks(frameElement, patch: patch) {
                masks.recomputeAll = true
            }
            frameElement.applyOcclusionPatch(patch)
            dirty.frame = true
            return
        }
        if let buttonElement = buttonElements.elements[patch.id] {
            let previousParentFrame = buttonElement.parentFrame
            buttonElement.applyOcclusionPatch(patch)
            if previousParentFrame != buttonElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.button = true
            return
        }
        if let checkboxElement = checkboxElements.elements[patch.id] {
            let previousParentFrame = checkboxElement.parentFrame
            checkboxElement.applyOcclusionPatch(patch)
            if previousParentFrame != checkboxElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.checkbox = true
            return
        }
        if let nativeImageElement = nativeImageElements.elements[patch.id] {
            let previousParentFrame = nativeImageElement.parentFrame
            nativeImageElement.applyOcclusionPatch(patch)
            if previousParentFrame != nativeImageElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.nativeImage = true
            return
        }
        if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
            let previousParentFrame = youtubeVideoElement.parentFrame
            youtubeVideoElement.applyOcclusionPatch(patch)
            if previousParentFrame != youtubeVideoElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.youtubeVideo = true
            return
        }
        if let dropdownElement = dropdownElements.elements[patch.id] {
            let previousParentFrame = dropdownElement.parentFrame
            dropdownElement.applyOcclusionPatch(patch)
            if previousParentFrame != dropdownElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.dropdown = true
            return
        }
        if let radioSetElement = radioSetElements.elements[patch.id] {
            let previousParentFrame = radioSetElement.parentFrame
            radioSetElement.applyOcclusionPatch(patch)
            if previousParentFrame != radioSetElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.radioSet = true
            return
        }
        if let sliderElement = sliderElements.elements[patch.id] {
            let previousParentFrame = sliderElement.parentFrame
            sliderElement.applyOcclusionPatch(patch)
            if previousParentFrame != sliderElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.slider = true
            return
        }
        if let textboxElement = textboxElements.elements[patch.id] {
            let previousParentFrame = textboxElement.parentFrame
            textboxElement.applyOcclusionPatch(patch)
            if previousParentFrame != textboxElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.textbox = true
            return
        }
        if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
            let previousParentFrame = eventBlockerElement.parentFrame
            eventBlockerElement.applyOcclusionPatch(patch)
            if previousParentFrame != eventBlockerElement.parentFrame {
                masks.mark(patch.id)
            }
            dirty.eventBlocker = true
        }
    }

    func handleNativeMaskUpdate(patch: NativeMaskPatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let textElement = textElements.elements[patch.id] {
            textElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.text = true
            return
        }
        if let buttonElement = buttonElements.elements[patch.id] {
            buttonElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.button = true
            return
        }
        if let checkboxElement = checkboxElements.elements[patch.id] {
            checkboxElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.checkbox = true
            return
        }
        if let nativeImageElement = nativeImageElements.elements[patch.id] {
            nativeImageElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.nativeImage = true
            return
        }
        if let youtubeVideoElement = youtubeVideoElements.elements[patch.id] {
            youtubeVideoElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.youtubeVideo = true
            return
        }
        if let dropdownElement = dropdownElements.elements[patch.id] {
            dropdownElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.dropdown = true
            return
        }
        if let radioSetElement = radioSetElements.elements[patch.id] {
            radioSetElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.radioSet = true
            return
        }
        if let sliderElement = sliderElements.elements[patch.id] {
            sliderElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.slider = true
            return
        }
        if let textboxElement = textboxElements.elements[patch.id] {
            textboxElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.textbox = true
            return
        }
        if let eventBlockerElement = eventBlockerElements.elements[patch.id] {
            eventBlockerElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.eventBlocker = true
        }
    }

    func processNativeMessageQueueData(_ data: Data) {
        guard let root = FlexBuffer.decode(data: data) else {
            return
        }
        var dirty = DirtyCollections()
        var masks = DirtyResolvedMasks()

        root["messages"]?.asVector?.makeIterator().forEach { message in
            if let textCreateMessage = message["TextCreate"] {
                handleTextCreate(patch: AnyCreatePatch(fb: textCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let textUpdateMessage = message["TextUpdate"] {
                handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let textDeleteMessage = message["TextDelete"] {
                handleTextDelete(patch: AnyDeletePatch(fb: textDeleteMessage), dirty: &dirty)
            }

            if let frameCreateMessage = message["FrameCreate"] {
                handleFrameCreate(patch: AnyCreatePatch(fb: frameCreateMessage), dirty: &dirty)
            }
            if let frameUpdateMessage = message["FrameUpdate"] {
                handleFrameUpdate(patch: FrameUpdatePatch(fb: frameUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let frameDeleteMessage = message["FrameDelete"] {
                handleFrameDelete(patch: AnyDeletePatch(fb: frameDeleteMessage), dirty: &dirty, masks: &masks)
            }

            if let buttonCreateMessage = message["ButtonCreate"] {
                handleButtonCreate(patch: AnyCreatePatch(fb: buttonCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let buttonUpdateMessage = message["ButtonUpdate"] {
                handleButtonUpdate(patch: ButtonUpdatePatch(fb: buttonUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let buttonDeleteMessage = message["ButtonDelete"] {
                handleButtonDelete(patch: AnyDeletePatch(fb: buttonDeleteMessage), dirty: &dirty)
            }

            if let checkboxCreateMessage = message["CheckboxCreate"] {
                handleCheckboxCreate(patch: AnyCreatePatch(fb: checkboxCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let checkboxUpdateMessage = message["CheckboxUpdate"] {
                handleCheckboxUpdate(patch: CheckboxUpdatePatch(fb: checkboxUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let checkboxDeleteMessage = message["CheckboxDelete"] {
                handleCheckboxDelete(patch: AnyDeletePatch(fb: checkboxDeleteMessage), dirty: &dirty)
            }

            if let nativeImageCreateMessage = message["NativeImageCreate"] {
                handleNativeImageCreate(patch: AnyCreatePatch(fb: nativeImageCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let nativeImageUpdateMessage = message["NativeImageUpdate"] {
                handleNativeImageUpdate(patch: NativeImageUpdatePatch(fb: nativeImageUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let nativeImageDeleteMessage = message["NativeImageDelete"] {
                handleNativeImageDelete(patch: AnyDeletePatch(fb: nativeImageDeleteMessage), dirty: &dirty)
            }

            if let youtubeVideoCreateMessage = message["YoutubeVideoCreate"] {
                handleYoutubeVideoCreate(patch: AnyCreatePatch(fb: youtubeVideoCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let youtubeVideoUpdateMessage = message["YoutubeVideoUpdate"] {
                handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch(fb: youtubeVideoUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let youtubeVideoDeleteMessage = message["YoutubeVideoDelete"] {
                handleYoutubeVideoDelete(patch: AnyDeletePatch(fb: youtubeVideoDeleteMessage), dirty: &dirty)
            }

            if let textboxCreateMessage = message["TextboxCreate"] {
                handleTextboxCreate(patch: AnyCreatePatch(fb: textboxCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let textboxUpdateMessage = message["TextboxUpdate"] {
                handleTextboxUpdate(patch: TextboxUpdatePatch(fb: textboxUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let textboxDeleteMessage = message["TextboxDelete"] {
                handleTextboxDelete(patch: AnyDeletePatch(fb: textboxDeleteMessage), dirty: &dirty)
            }

            if let sliderCreateMessage = message["SliderCreate"] {
                handleSliderCreate(patch: AnyCreatePatch(fb: sliderCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let sliderUpdateMessage = message["SliderUpdate"] {
                handleSliderUpdate(patch: SliderUpdatePatch(fb: sliderUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let sliderDeleteMessage = message["SliderDelete"] {
                handleSliderDelete(patch: AnyDeletePatch(fb: sliderDeleteMessage), dirty: &dirty)
            }

            if let dropdownCreateMessage = message["DropdownCreate"] {
                handleDropdownCreate(patch: AnyCreatePatch(fb: dropdownCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let dropdownUpdateMessage = message["DropdownUpdate"] {
                handleDropdownUpdate(patch: DropdownUpdatePatch(fb: dropdownUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let dropdownDeleteMessage = message["DropdownDelete"] {
                handleDropdownDelete(patch: AnyDeletePatch(fb: dropdownDeleteMessage), dirty: &dirty)
            }

            if let radioSetCreateMessage = message["RadioSetCreate"] {
                handleRadioSetCreate(patch: AnyCreatePatch(fb: radioSetCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let radioSetUpdateMessage = message["RadioSetUpdate"] {
                handleRadioSetUpdate(patch: RadioSetUpdatePatch(fb: radioSetUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let radioSetDeleteMessage = message["RadioSetDelete"] {
                handleRadioSetDelete(patch: AnyDeletePatch(fb: radioSetDeleteMessage), dirty: &dirty)
            }

            if let eventBlockerCreateMessage = message["EventBlockerCreate"] {
                handleEventBlockerCreate(patch: AnyCreatePatch(fb: eventBlockerCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let eventBlockerUpdateMessage = message["EventBlockerUpdate"] {
                handleEventBlockerUpdate(patch: EventBlockerPatchMessage(fb: eventBlockerUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let eventBlockerDeleteMessage = message["EventBlockerDelete"] {
                handleEventBlockerDelete(patch: AnyDeletePatch(fb: eventBlockerDeleteMessage), dirty: &dirty)
            }

            if let occlusionUpdateMessage = message["OcclusionUpdate"] {
                handleOcclusionUpdate(patch: OcclusionUpdatePatch(fb: occlusionUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let nativeMaskUpdateMessage = message["NativeMaskUpdate"] {
                handleNativeMaskUpdate(patch: NativeMaskPatch(fb: nativeMaskUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let imageLoadMessage = message["ImageLoad"] {
                handleImageLoad(patch: ImageLoadPatch(fb: imageLoadMessage))
            }
            if let navigateMessage = message["Navigate"] {
                handleNavigate(patch: NavigationPatchMessage(fb: navigateMessage))
            }

            let _ = message["SetCursor"]
            let _ = message["LayerAdd"]
            let _ = message["ShrinkLayersTo"]
            let _ = message["ScrollerCreate"]
            let _ = message["ScrollerUpdate"]
            let _ = message["ScrollerDelete"]
        }

        if masks.recomputeAll {
            recomputeAllResolvedMasks()
        } else {
            recomputeResolvedMasks(for: masks.elementIds)
        }
        publish(dirty)
    }
}
