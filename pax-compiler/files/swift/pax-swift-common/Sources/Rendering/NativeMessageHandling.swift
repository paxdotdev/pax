import Foundation
import SwiftUI
import FlexBuffers
import Messages
#if os(macOS)
import AppKit
#endif

public struct DirtyCollections {
    public var text = false
    public var frame = false
    public var scroller = false
    public var button = false
    public var photoPicker = false
    public var checkbox = false
    public var nativeImage = false
    public var youtubeVideo = false
    public var dropdown = false
    public var radioList = false
    public var slider = false
    public var textbox = false
    public var eventBlocker = false
    public var glassSurface = false

    public var hasAny: Bool {
        text || frame || scroller || button || photoPicker || checkbox || nativeImage || youtubeVideo || dropdown || radioList || slider || textbox || eventBlocker || glassSurface
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
    var photoPickerElements: PhotoPickerElements { get }
    var checkboxElements: CheckboxElements { get }
    var scrollerElements: ScrollerElements { get }
    var nativeImageElements: NativeImageElements { get }
    var youtubeVideoElements: YoutubeVideoElements { get }
    var dropdownElements: DropdownElements { get }
    var radioListElements: RadioListElements { get }
    var sliderElements: SliderElements { get }
    var textboxElements: TextboxElements { get }
    var eventBlockerElements: EventBlockerElements { get }
    var glassSurfaceElements: GlassSurfaceElements { get }

    func handleImageLoad(patch: ImageLoadPatch)
    func handleNavigate(patch: NavigationPatchMessage)
    func didUpdateTextElement(_ textElement: TextElement, measureGeneration: UInt64?)
}

public extension NativeMessageHandling {
    func handleSetCursor(patch: SetCursorPatchMessage) {
        #if os(macOS)
        let cursor: NSCursor
        switch patch.cursor {
        case "pointer":
            cursor = .pointingHand
        case "text", "vertical-text":
            cursor = .iBeam
        case "crosshair":
            cursor = .crosshair
        case "not-allowed", "no-drop":
            cursor = .operationNotAllowed
        case "grab":
            cursor = .openHand
        case "grabbing":
            cursor = .closedHand
        case "col-resize", "e-resize", "w-resize", "ew-resize":
            cursor = .resizeLeftRight
        case "row-resize", "n-resize", "s-resize", "ns-resize":
            cursor = .resizeUpDown
        default:
            cursor = .arrow
        }
        DispatchQueue.main.async {
            cursor.set()
        }
        #endif
    }

    private func hasScrollerScrollUpdate(_ patch: ScrollerUpdatePatch) -> Bool {
        patch.scroll_x != nil
            || patch.scroll_y != nil
            || patch.presentation_scroll_x != nil
            || patch.presentation_scroll_y != nil
    }

    private func applyScrollOnlyScrollerPatch(_ scroller: ScrollerElement) -> Bool {
        let scrollX = scroller.presentationScrollX.isFinite
            ? scroller.presentationScrollX
            : scroller.scrollX
        let scrollY = scroller.presentationScrollY.isFinite
            ? scroller.presentationScrollY
            : scroller.scrollY
        return NativeScrollerHostRegistry.shared.updateScrollPosition(
            id: scroller.id,
            position: CGPoint(x: scrollX, y: scrollY)
        )
    }

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

    private func recomputeResolvedMaskIfPresent(id: PaxNodeId) {
        if let textElement = textElements.elements[id] {
            recomputeResolvedMask(for: textElement)
            return
        }
        if let element = buttonElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = photoPickerElements.elements[id] {
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
        if let element = radioListElements.elements[id] {
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
            return
        }
        if let element = glassSurfaceElements.elements[id] {
            recomputeResolvedMask(for: element)
            return
        }
        if let element = scrollerElements.elements[id] {
            recomputeResolvedMask(for: element)
        }
    }

    private func recomputeResolvedMasks(for ids: Set<PaxNodeId>) {
        for id in ids {
            recomputeResolvedMaskIfPresent(id: id)
        }
    }

    func didUpdateTextElement(_ textElement: TextElement, measureGeneration: UInt64?) {
        _ = textElement
        _ = measureGeneration
    }

    func resolvedTextMaskSize(_ textElement: TextElement) -> CGSize {
        let width = textElement.size_x >= 0 ? CGFloat(textElement.size_x) : textElement.lastMeasuredSize?.width ?? 0
        let height = textElement.size_y >= 0 ? CGFloat(textElement.size_y) : textElement.lastMeasuredSize?.height ?? 0
        return CGSize(width: max(0, width), height: max(0, height))
    }

    func recomputeResolvedMask(for textElement: TextElement) {
        let mask = resolveNativeMask(
            patch: textElement.nativeMaskPatch,
            fallbackSize: resolvedTextMaskSize(textElement)
        )
        setResolvedNativeMask(id: textElement.id, mask: mask)
    }

    func recomputeResolvedMask<T: NativePositionElement>(for element: T) {
        let mask = resolveNativeMask(
            patch: element.nativeMaskPatch,
            fallbackSize: CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y)))
        )
        setResolvedNativeMask(id: element.id, mask: mask)
    }

    func recomputeResolvedMask(for scrollerElement: ScrollerElement) {
        let mask = resolveNativeMask(
            patch: scrollerElement.nativeMaskPatch,
            fallbackSize: CGSize(width: max(0, CGFloat(scrollerElement.size_x)), height: max(0, CGFloat(scrollerElement.size_y)))
        )
        setResolvedNativeMask(id: scrollerElement.id, mask: mask)
        NativeScrollerHostRegistry.shared.setCanvasOpacityMultiplier(
            id: scrollerElement.id,
            multiplier: splitNativeMaskFullCoverageAttenuation(mask).opacityMultiplier
        )
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
        recomputeResolvedMasks(in: photoPickerElements.elements)
        recomputeResolvedMasks(in: checkboxElements.elements)
        recomputeResolvedMasks(in: nativeImageElements.elements)
        recomputeResolvedMasks(in: youtubeVideoElements.elements)
        recomputeResolvedMasks(in: dropdownElements.elements)
        recomputeResolvedMasks(in: radioListElements.elements)
        recomputeResolvedMasks(in: sliderElements.elements)
        recomputeResolvedMasks(in: textboxElements.elements)
        recomputeResolvedMasks(in: eventBlockerElements.elements)
        recomputeResolvedMasks(in: glassSurfaceElements.elements)
        for scrollerElement in scrollerElements.elements.values {
            recomputeResolvedMask(for: scrollerElement)
        }
    }

    func publish(_ dirty: DirtyCollections) {
        guard dirty.hasAny else {
            return
        }
        NativeSceneInvalidation.singleton.invalidate()
    }

    func handleTextCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        textElements.add(element: TextElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.text = true
    }

    func handleTextUpdate(patch: TextUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let textElement = textElements.elements[patch.id] {
            let previousTransform = textElement.transform
            let previousSizeX = textElement.size_x
            let previousSizeY = textElement.size_y
            textElement.applyPatch(patch: patch)
            textElement.applyResolvedPlacement(patch)
            if textGeometryChanged(
                textElement,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
            didUpdateTextElement(textElement, measureGeneration: patch.measureGeneration)
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

    func handleFrameUpdate(patch: FrameUpdatePatch, dirty: inout DirtyCollections, masks _: inout DirtyResolvedMasks) {
        if let frame = frameElements.elements[patch.id] {
            frame.applyPatch(patch: patch)
            frame.applyResolvedPlacement(patch)
        }
        dirty.frame = true
    }

    func handleScrollerCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        scrollerElements.add(element: ScrollerElement.makeDefault(
            id: patch.id,
            parentFrame: patch.parentFrame,
            renderLayerId: patch.renderLayerId
        ))
        masks.mark(patch.id)
        dirty.scroller = true
    }

    func handleScrollerUpdate(patch: ScrollerUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        let hasScrollUpdate = hasScrollerScrollUpdate(patch)
        if let scroller = scrollerElements.elements[patch.id] {
            let previousParentFrame = scroller.parentFrame
            let previousZIndex = scroller.zIndex
            let previousTransform = scroller.transform
            let previousSizeX = scroller.size_x
            let previousSizeY = scroller.size_y
            let previousOpacity = scroller.opacity
            let previousClipContent = scroller.clipContent
            let previousBorderRadius = scroller.borderRadius
            let previousSizeInnerPaneX = scroller.sizeInnerPaneX
            let previousSizeInnerPaneY = scroller.sizeInnerPaneY
            let previousSnapPointsX = scroller.snapPointsX
            let previousSnapPointsY = scroller.snapPointsY
            let previousScrollEnabledX = scroller.scrollEnabledX
            let previousScrollEnabledY = scroller.scrollEnabledY
            let previousContentLayerId = scroller.contentLayerId
            let previousPresentedBounds = scroller.presentedBounds
            let previousPresentedClipBounds = scroller.presentedClipBounds
            let previousSubtreeDepth = scroller.subtreeDepth
            scroller.applyPatch(patch: patch)
            scroller.applyResolvedPlacement(patch)
            let scrollerGeometryChanged = geometryChanged(
                scroller,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            )
            let structuralChanged = scrollerGeometryChanged
                || previousParentFrame != scroller.parentFrame
                || previousZIndex != scroller.zIndex
                || previousOpacity != scroller.opacity
                || previousClipContent != scroller.clipContent
                || previousBorderRadius != scroller.borderRadius
                || previousSizeInnerPaneX != scroller.sizeInnerPaneX
                || previousSizeInnerPaneY != scroller.sizeInnerPaneY
                || previousSnapPointsX != scroller.snapPointsX
                || previousSnapPointsY != scroller.snapPointsY
                || previousScrollEnabledX != scroller.scrollEnabledX
                || previousScrollEnabledY != scroller.scrollEnabledY
                || previousContentLayerId != scroller.contentLayerId
                || previousPresentedBounds != scroller.presentedBounds
                || previousPresentedClipBounds != scroller.presentedClipBounds
                || previousSubtreeDepth != scroller.subtreeDepth

            if hasScrollUpdate && !structuralChanged {
                if !applyScrollOnlyScrollerPatch(scroller) {
                    dirty.scroller = true
                }
                return
            }
            if scrollerGeometryChanged {
                masks.mark(patch.id)
            }
        }
        dirty.scroller = true
    }

    func handleScrollerDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        scrollerElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        NativeScrollerHostRegistry.shared.setCanvasOpacityMultiplier(id: patch.id, multiplier: 1.0)
        dirty.scroller = true
    }

    func handleFrameDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections, masks _: inout DirtyResolvedMasks) {
        frameElements.remove(id: patch.id)
        dirty.frame = true
    }

    func handleButtonCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        buttonElements.add(element: ButtonElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.button = true
    }

    func handleButtonUpdate(patch: ButtonUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = buttonElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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

    func handlePhotoPickerCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        photoPickerElements.add(element: PhotoPickerElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.photoPicker = true
    }

    func handlePhotoPickerUpdate(patch: PhotoPickerUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = photoPickerElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.photoPicker = true
    }

    func handlePhotoPickerDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        photoPickerElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.photoPicker = true
    }

    func handleCheckboxCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        checkboxElements.add(element: CheckboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.checkbox = true
    }

    func handleCheckboxUpdate(patch: CheckboxUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = checkboxElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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
        nativeImageElements.add(element: NativeImageElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.nativeImage = true
    }

    func handleNativeImageUpdate(patch: NativeImageUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = nativeImageElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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
        youtubeVideoElements.add(element: YoutubeVideoElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.youtubeVideo = true
    }

    func handleYoutubeVideoUpdate(patch: YoutubeVideoUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = youtubeVideoElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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
        dropdownElements.add(element: DropdownElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.dropdown = true
    }

    func handleDropdownUpdate(patch: DropdownUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = dropdownElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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

    func handleRadioListCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        radioListElements.add(element: RadioListElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.radioList = true
    }

    func handleRadioListUpdate(patch: RadioListUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = radioListElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.radioList = true
    }

    func handleRadioListDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        radioListElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.radioList = true
    }

    func handleSliderCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        sliderElements.add(element: SliderElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.slider = true
    }

    func handleSliderUpdate(patch: SliderUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = sliderElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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
        textboxElements.add(element: TextboxElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.textbox = true
    }

    func handleTextboxUpdate(patch: TextboxUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = textboxElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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
        eventBlockerElements.add(element: EventBlockerElement.makeDefault(id: patch.id, parentFrame: patch.parentFrame, renderLayerId: patch.renderLayerId))
        masks.mark(patch.id)
        dirty.eventBlocker = true
    }

    func handleEventBlockerUpdate(patch: EventBlockerPatchMessage, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = eventBlockerElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
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

    func handleGlassSurfaceCreate(patch: AnyCreatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        glassSurfaceElements.add(element: GlassSurfaceElement.makeDefault(
            id: patch.id,
            parentFrame: patch.parentFrame,
            renderLayerId: patch.renderLayerId
        ))
        masks.mark(patch.id)
        dirty.glassSurface = true
    }

    func handleGlassSurfaceUpdate(patch: GlassSurfaceUpdatePatch, dirty: inout DirtyCollections, masks: inout DirtyResolvedMasks) {
        if let element = glassSurfaceElements.elements[patch.id] {
            let previousTransform = element.transform
            let previousSizeX = element.size_x
            let previousSizeY = element.size_y
            element.applyPatch(patch)
            element.applyResolvedPlacement(patch)
            if geometryChanged(
                element,
                previousTransform: previousTransform,
                previousSizeX: previousSizeX,
                previousSizeY: previousSizeY
            ) {
                masks.mark(patch.id)
            }
        }
        dirty.glassSurface = true
    }

    func handleGlassSurfaceDelete(patch: AnyDeletePatch, dirty: inout DirtyCollections) {
        glassSurfaceElements.remove(id: patch.id)
        removeResolvedNativeMask(id: patch.id)
        dirty.glassSurface = true
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
        if let photoPickerElement = photoPickerElements.elements[patch.id] {
            photoPickerElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.photoPicker = true
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
        if let radioListElement = radioListElements.elements[patch.id] {
            radioListElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.radioList = true
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
            return
        }
        if let glassSurfaceElement = glassSurfaceElements.elements[patch.id] {
            glassSurfaceElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.glassSurface = true
            return
        }
        if let scrollerElement = scrollerElements.elements[patch.id] {
            scrollerElement.applyNativeMaskPatch(patch)
            masks.mark(patch.id)
            dirty.scroller = true
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

            if let scrollerCreateMessage = message["ScrollerCreate"] {
                handleScrollerCreate(patch: AnyCreatePatch(fb: scrollerCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let scrollerUpdateMessage = message["ScrollerUpdate"] {
                handleScrollerUpdate(patch: ScrollerUpdatePatch(fb: scrollerUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let scrollerDeleteMessage = message["ScrollerDelete"] {
                handleScrollerDelete(patch: AnyDeletePatch(fb: scrollerDeleteMessage), dirty: &dirty)
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

            if let photoPickerCreateMessage = message["PhotoPickerCreate"] {
                handlePhotoPickerCreate(patch: AnyCreatePatch(fb: photoPickerCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let photoPickerUpdateMessage = message["PhotoPickerUpdate"] {
                handlePhotoPickerUpdate(patch: PhotoPickerUpdatePatch(fb: photoPickerUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let photoPickerDeleteMessage = message["PhotoPickerDelete"] {
                handlePhotoPickerDelete(patch: AnyDeletePatch(fb: photoPickerDeleteMessage), dirty: &dirty)
            }

            if let glassSurfaceCreateMessage = message["GlassSurfaceCreate"] {
                handleGlassSurfaceCreate(patch: AnyCreatePatch(fb: glassSurfaceCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let glassSurfaceUpdateMessage = message["GlassSurfaceUpdate"] {
                handleGlassSurfaceUpdate(patch: GlassSurfaceUpdatePatch(fb: glassSurfaceUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let glassSurfaceDeleteMessage = message["GlassSurfaceDelete"] {
                handleGlassSurfaceDelete(patch: AnyDeletePatch(fb: glassSurfaceDeleteMessage), dirty: &dirty)
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

            if let radioListCreateMessage = message["RadioListCreate"] {
                handleRadioListCreate(patch: AnyCreatePatch(fb: radioListCreateMessage), dirty: &dirty, masks: &masks)
            }
            if let radioListUpdateMessage = message["RadioListUpdate"] {
                handleRadioListUpdate(patch: RadioListUpdatePatch(fb: radioListUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let radioListDeleteMessage = message["RadioListDelete"] {
                handleRadioListDelete(patch: AnyDeletePatch(fb: radioListDeleteMessage), dirty: &dirty)
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

            if let nativeMaskUpdateMessage = message["NativeMaskUpdate"] {
                handleNativeMaskUpdate(patch: NativeMaskPatch(fb: nativeMaskUpdateMessage), dirty: &dirty, masks: &masks)
            }
            if let imageLoadMessage = message["ImageLoad"] {
                handleImageLoad(patch: ImageLoadPatch(fb: imageLoadMessage))
            }
            if let navigateMessage = message["Navigate"] {
                handleNavigate(patch: NavigationPatchMessage(fb: navigateMessage))
            }

            if let shrinkLayersMessage = message["ShrinkLayersTo"] {
                if let count = shrinkLayersMessage.asUInt64 {
                    NativeLayerCountTracker.shared.update(Int(count))
                } else if let count = shrinkLayersMessage.asInt {
                    NativeLayerCountTracker.shared.update(Int(count))
                }
            }

            if let setCursorMessage = message["SetCursor"] {
                handleSetCursor(patch: SetCursorPatchMessage(fb: setCursorMessage))
            }
        }

        if masks.recomputeAll {
            recomputeAllResolvedMasks()
        } else {
            recomputeResolvedMasks(for: masks.elementIds)
        }
        publish(dirty)
    }
}
