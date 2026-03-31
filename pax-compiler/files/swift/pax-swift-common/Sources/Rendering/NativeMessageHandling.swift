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

    func processNativeMessageQueueData(_ data: Data) {
        guard let root = FlexBuffer.decode(data: data) else {
            return
        }
        var dirty = DirtyCollections()
        var needsFrameMaskRecompute = false

        root["messages"]?.asVector?.makeIterator().forEach { message in
            if let textCreateMessage = message["TextCreate"] {
                handleTextCreate(patch: AnyCreatePatch(fb: textCreateMessage), dirty: &dirty)
            }
            if let textUpdateMessage = message["TextUpdate"] {
                handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage), dirty: &dirty)
            }
            if let textDeleteMessage = message["TextDelete"] {
                handleTextDelete(patch: AnyDeletePatch(fb: textDeleteMessage), dirty: &dirty)
            }

            if let frameCreateMessage = message["FrameCreate"] {
                handleFrameCreate(patch: AnyCreatePatch(fb: frameCreateMessage), dirty: &dirty)
                needsFrameMaskRecompute = true
            }
            if let frameUpdateMessage = message["FrameUpdate"] {
                handleFrameUpdate(patch: FrameUpdatePatch(fb: frameUpdateMessage), dirty: &dirty)
                needsFrameMaskRecompute = true
            }
            if let frameDeleteMessage = message["FrameDelete"] {
                handleFrameDelete(patch: AnyDeletePatch(fb: frameDeleteMessage), dirty: &dirty)
                needsFrameMaskRecompute = true
            }

            if let buttonCreateMessage = message["ButtonCreate"] {
                handleButtonCreate(patch: AnyCreatePatch(fb: buttonCreateMessage), dirty: &dirty)
            }
            if let buttonUpdateMessage = message["ButtonUpdate"] {
                handleButtonUpdate(patch: ButtonUpdatePatch(fb: buttonUpdateMessage), dirty: &dirty)
            }
            if let buttonDeleteMessage = message["ButtonDelete"] {
                handleButtonDelete(patch: AnyDeletePatch(fb: buttonDeleteMessage), dirty: &dirty)
            }

            if let checkboxCreateMessage = message["CheckboxCreate"] {
                handleCheckboxCreate(patch: AnyCreatePatch(fb: checkboxCreateMessage), dirty: &dirty)
            }
            if let checkboxUpdateMessage = message["CheckboxUpdate"] {
                handleCheckboxUpdate(patch: CheckboxUpdatePatch(fb: checkboxUpdateMessage), dirty: &dirty)
            }
            if let checkboxDeleteMessage = message["CheckboxDelete"] {
                handleCheckboxDelete(patch: AnyDeletePatch(fb: checkboxDeleteMessage), dirty: &dirty)
            }

            if let nativeImageCreateMessage = message["NativeImageCreate"] {
                handleNativeImageCreate(patch: AnyCreatePatch(fb: nativeImageCreateMessage), dirty: &dirty)
            }
            if let nativeImageUpdateMessage = message["NativeImageUpdate"] {
                handleNativeImageUpdate(patch: NativeImageUpdatePatch(fb: nativeImageUpdateMessage), dirty: &dirty)
            }
            if let nativeImageDeleteMessage = message["NativeImageDelete"] {
                handleNativeImageDelete(patch: AnyDeletePatch(fb: nativeImageDeleteMessage), dirty: &dirty)
            }

            if let youtubeVideoCreateMessage = message["YoutubeVideoCreate"] {
                handleYoutubeVideoCreate(patch: AnyCreatePatch(fb: youtubeVideoCreateMessage), dirty: &dirty)
            }
            if let youtubeVideoUpdateMessage = message["YoutubeVideoUpdate"] {
                handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch(fb: youtubeVideoUpdateMessage), dirty: &dirty)
            }
            if let youtubeVideoDeleteMessage = message["YoutubeVideoDelete"] {
                handleYoutubeVideoDelete(patch: AnyDeletePatch(fb: youtubeVideoDeleteMessage), dirty: &dirty)
            }

            if let textboxCreateMessage = message["TextboxCreate"] {
                handleTextboxCreate(patch: AnyCreatePatch(fb: textboxCreateMessage), dirty: &dirty)
            }
            if let textboxUpdateMessage = message["TextboxUpdate"] {
                handleTextboxUpdate(patch: TextboxUpdatePatch(fb: textboxUpdateMessage), dirty: &dirty)
            }
            if let textboxDeleteMessage = message["TextboxDelete"] {
                handleTextboxDelete(patch: AnyDeletePatch(fb: textboxDeleteMessage), dirty: &dirty)
            }

            if let sliderCreateMessage = message["SliderCreate"] {
                handleSliderCreate(patch: AnyCreatePatch(fb: sliderCreateMessage), dirty: &dirty)
            }
            if let sliderUpdateMessage = message["SliderUpdate"] {
                handleSliderUpdate(patch: SliderUpdatePatch(fb: sliderUpdateMessage), dirty: &dirty)
            }
            if let sliderDeleteMessage = message["SliderDelete"] {
                handleSliderDelete(patch: AnyDeletePatch(fb: sliderDeleteMessage), dirty: &dirty)
            }

            if let dropdownCreateMessage = message["DropdownCreate"] {
                handleDropdownCreate(patch: AnyCreatePatch(fb: dropdownCreateMessage), dirty: &dirty)
            }
            if let dropdownUpdateMessage = message["DropdownUpdate"] {
                handleDropdownUpdate(patch: DropdownUpdatePatch(fb: dropdownUpdateMessage), dirty: &dirty)
            }
            if let dropdownDeleteMessage = message["DropdownDelete"] {
                handleDropdownDelete(patch: AnyDeletePatch(fb: dropdownDeleteMessage), dirty: &dirty)
            }

            if let radioSetCreateMessage = message["RadioSetCreate"] {
                handleRadioSetCreate(patch: AnyCreatePatch(fb: radioSetCreateMessage), dirty: &dirty)
            }
            if let radioSetUpdateMessage = message["RadioSetUpdate"] {
                handleRadioSetUpdate(patch: RadioSetUpdatePatch(fb: radioSetUpdateMessage), dirty: &dirty)
            }
            if let radioSetDeleteMessage = message["RadioSetDelete"] {
                handleRadioSetDelete(patch: AnyDeletePatch(fb: radioSetDeleteMessage), dirty: &dirty)
            }

            if let eventBlockerCreateMessage = message["EventBlockerCreate"] {
                handleEventBlockerCreate(patch: AnyCreatePatch(fb: eventBlockerCreateMessage), dirty: &dirty)
            }
            if let eventBlockerUpdateMessage = message["EventBlockerUpdate"] {
                handleEventBlockerUpdate(patch: EventBlockerPatchMessage(fb: eventBlockerUpdateMessage), dirty: &dirty)
            }
            if let eventBlockerDeleteMessage = message["EventBlockerDelete"] {
                handleEventBlockerDelete(patch: AnyDeletePatch(fb: eventBlockerDeleteMessage), dirty: &dirty)
            }

            if let occlusionUpdateMessage = message["OcclusionUpdate"] {
                handleOcclusionUpdate(patch: OcclusionUpdatePatch(fb: occlusionUpdateMessage), dirty: &dirty)
            }
            if let nativeMaskUpdateMessage = message["NativeMaskUpdate"] {
                handleNativeMaskUpdate(patch: NativeMaskPatch(fb: nativeMaskUpdateMessage), dirty: &dirty)
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

        if needsFrameMaskRecompute {
            recomputeAllResolvedMasks()
        }
        publish(dirty)
    }
}
