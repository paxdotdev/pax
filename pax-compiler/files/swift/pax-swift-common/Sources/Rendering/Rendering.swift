import SwiftUI
import Messages
#if os(iOS) || os(tvOS) || os(watchOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

private func affineTransform(from coeffs: [Float]) -> CGAffineTransform {
    guard coeffs.count >= 6 else {
        return .identity
    }
    return CGAffineTransform(
        a: CGFloat(coeffs[0]),
        b: CGFloat(coeffs[1]),
        c: CGFloat(coeffs[2]),
        d: CGFloat(coeffs[3]),
        tx: CGFloat(coeffs[4]),
        ty: CGFloat(coeffs[5])
    )
}

private func resolvedDimension(_ value: Float) -> CGFloat? {
    guard value >= 0 else {
        return nil
    }
    return CGFloat(value)
}

private func resolvedSize(_ element: NativePositionElement) -> CGSize {
    CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y)))
}

private func resolvedTextSize(_ element: TextElement) -> CGSize {
    CGSize(width: max(0, CGFloat(element.size_x)), height: max(0, CGFloat(element.size_y)))
}

private func textAlignment(from alignment: TextAlignment) -> TextAlignment {
    alignment
}

#if os(iOS) || os(tvOS) || os(watchOS)
private func platformColor(_ color: Color) -> UIColor {
    UIColor(color)
}

private func platformTextAlignment(_ alignment: TextAlignment) -> NSTextAlignment {
    switch alignment {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}
#elseif os(macOS)
private func platformColor(_ color: Color) -> NSColor {
    NSColor(color)
}

private func platformTextAlignment(_ alignment: TextAlignment) -> NSTextAlignment {
    switch alignment {
    case .center:
        return .center
    case .trailing:
        return .right
    default:
        return .left
    }
}
#endif

public struct NativeRenderingLayer: View {
    public init() {}

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

    private func sortedTextElements() -> [TextElement] {
        Array(textElements.elements.values).sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.id < rhs.id
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func sortedElements<T: NativePositionElement>(_ elements: [PaxNodeId: T]) -> [T] {
        Array(elements.values).sorted { lhs, rhs in
            if lhs.zIndex == rhs.zIndex {
                return lhs.id < rhs.id
            }
            return lhs.zIndex < rhs.zIndex
        }
    }

    private func applyNativeMask<V: View>(_ view: V, elementId: PaxNodeId) -> AnyView {
        guard let mask = resolvedNativeMask(for: elementId) else {
            return AnyView(view)
        }
        return AnyView(
            view
                .compositingGroup()
                .mask(CombinedMaskView(mask: mask))
        )
    }

    private func positioned<V: View>(_ view: V, element: NativePositionElement) -> AnyView {
        let size = resolvedSize(element)
        let bounded = view
            .frame(width: resolvedDimension(element.size_x), height: resolvedDimension(element.size_y))
            .clipped()
        let localMasked = applyNativeMask(bounded, elementId: element.id)
        let base = localMasked
            .position(x: size.width / 2.0, y: size.height / 2.0)
            .transformEffect(affineTransform(from: element.transform))
            .zIndex(Double(element.zIndex))
        return AnyView(base)
    }

    private func positionedText<V: View>(_ view: V, element: TextElement, width: CGFloat, height: CGFloat) -> AnyView {
        let bounded = view
            .frame(width: width > 0 ? width : nil, height: height > 0 ? height : nil)
            .clipped()
        let localMasked = applyNativeMask(bounded, elementId: element.id)
        let base = localMasked
            .position(x: width / 2.0, y: height / 2.0)
            .transformEffect(affineTransform(from: element.transform))
            .zIndex(Double(element.zIndex))
        return AnyView(base)
    }

    private func attributedString(for element: TextElement) -> AttributedString {
        var attributedString: AttributedString
        if element.markdown {
            attributedString = (try? AttributedString(
                markdown: element.content,
                options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
            )) ?? AttributedString(element.content)
        } else {
            attributedString = AttributedString(element.content)
        }

        for run in attributedString.runs {
            if run.link != nil, let linkStyle = element.style_link {
                attributedString[run.range].font = linkStyle.font.getFont(size: linkStyle.font_size)
                attributedString[run.range].underlineStyle = linkStyle.underline ? .single : .none
                attributedString[run.range].foregroundColor = linkStyle.fill
            }
        }

        return attributedString
    }

    private func textView(for element: TextElement) -> AnyView {
        let measuredWidth = element.size_x >= 0 ? CGFloat(element.size_x) : element.lastMeasuredSize?.width ?? 0
        let measuredHeight = element.size_y >= 0 ? CGFloat(element.size_y) : element.lastMeasuredSize?.height ?? 0

        if element.editable {
            return positionedText(
                EditableTextView(element: element),
                element: element,
                width: measuredWidth,
                height: measuredHeight
            )
        }

        let baseText = Text(attributedString(for: element))
            .foregroundColor(element.textStyle.fill)
            .font(element.textStyle.font.getFont(size: element.textStyle.font_size))
            .frame(
                width: measuredWidth > 0 ? measuredWidth : nil,
                height: measuredHeight > 0 ? measuredHeight : nil,
                alignment: element.textStyle.alignment
            )
            .allowsHitTesting(element.selectable)

        let text: AnyView
        if element.selectable {
            text = AnyView(baseText.textSelection(.enabled))
        } else {
            text = AnyView(baseText.textSelection(.disabled))
        }

        return positionedText(text, element: element, width: measuredWidth, height: measuredHeight)
    }

    private func buttonView(for element: ButtonElement) -> AnyView {
        let button = Button(action: {
            dispatchFormButtonClick(id: element.id)
        }) {
            Text(element.content.isEmpty ? " " : element.content)
                .font(element.style.font.getFont(size: element.style.font_size))
                .foregroundColor(element.style.fill)
                .multilineTextAlignment(textAlignment(from: element.style.alignmentMultiline))
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: element.style.alignment)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
        }
        .buttonStyle(.plain)
        .background(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .fill(element.color)
        )
        .overlay(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .stroke(element.outlineStrokeColor, lineWidth: element.outlineStrokeWidth)
        )

        return positioned(button, element: element)
    }

    private func checkboxView(for element: CheckboxElement) -> AnyView {
        let checkbox = Button(action: {
            dispatchFormCheckboxToggle(id: element.id, state: !element.checked)
        }) {
            ZStack {
                RoundedRectangle(cornerRadius: element.borderRadius)
                    .fill(element.checked ? element.backgroundChecked : element.background)
                RoundedRectangle(cornerRadius: element.borderRadius)
                    .stroke(element.outlineColor, lineWidth: element.outlineWidth)
                if element.checked {
                    Image(systemName: "checkmark")
                        .foregroundColor(.white)
                        .font(.system(size: max(10, min(resolvedSize(element).width, resolvedSize(element).height) * 0.55)))
                }
            }
        }
        .buttonStyle(.plain)

        return positioned(checkbox, element: element)
    }

    private func sliderView(for element: SliderElement) -> AnyView {
        let upperBound = element.max > element.min ? element.max : element.min + 1
        let step = element.step > 0 ? element.step : 0.001

        let slider = Slider(
            value: Binding(
                get: { element.value },
                set: { dispatchFormSliderChange(id: element.id, value: $0) }
            ),
            in: element.min...upperBound,
            step: step
        )
        .tint(element.accent)

        return positioned(ZStack { slider }, element: element)
    }

    private func dropdownView(for element: DropdownElement) -> AnyView {
        let picker = Picker(
            "",
            selection: Binding(
                get: { Int(element.selectedId) },
                set: { dispatchFormDropdownChange(id: element.id, selectedId: UInt32($0)) }
            )
        ) {
            ForEach(Array(element.options.enumerated()), id: \.offset) { index, option in
                Text(option)
                    .font(element.style.font.getFont(size: element.style.font_size))
                    .tag(index)
            }
        }
        .pickerStyle(.menu)
        .labelsHidden()
        .tint(element.style.fill)
        .background(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .fill(element.background)
        )
        .overlay(
            RoundedRectangle(cornerRadius: element.borderRadius)
                .stroke(element.strokeColor, lineWidth: element.strokeWidth)
        )

        return positioned(picker, element: element)
    }

    private func radioSetView(for element: RadioSetElement) -> AnyView {
        let control = VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(element.options.enumerated()), id: \.offset) { index, option in
                Button(action: {
                    dispatchFormRadioSetChange(id: element.id, selectedId: UInt32(index))
                }) {
                    HStack(spacing: 8) {
                        ZStack {
                            Circle()
                                .fill(element.background)
                            Circle()
                                .stroke(element.outlineColor, lineWidth: element.outlineWidth)
                            if Int(element.selectedId) == index {
                                Circle()
                                    .fill(element.backgroundChecked)
                                    .padding(4)
                            }
                        }
                        .frame(width: 20, height: 20)

                        Text(option)
                            .font(element.style.font.getFont(size: element.style.font_size))
                            .foregroundColor(element.style.fill)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                .buttonStyle(.plain)
            }
        }

        return positioned(control, element: element)
    }

    private func textboxView(for element: TextboxElement) -> AnyView {
        if element.isTextArea {
            return positioned(PaxTextboxArea(element: element), element: element)
        }
        return positioned(PaxTextboxField(element: element), element: element)
    }

    private func nativeImageView(for element: NativeImageElement) -> AnyView {
        let imageView = Group {
            if let url = URL(string: element.url), url.scheme != nil {
                AsyncImage(url: url) { phase in
                    switch phase {
                    case .success(let image):
                        configuredImage(image, fit: element.fit)
                    default:
                        Color.clear
                    }
                }
            } else if !element.url.isEmpty {
                if let platformImage = loadPlatformImage(path: element.url) {
                    configuredPlatformImage(platformImage, fit: element.fit)
                } else {
                    Color.clear
                }
            } else {
                Color.clear
            }
        }

        return positioned(imageView, element: element)
    }

    private func youtubeVideoView(for element: YoutubeVideoElement) -> AnyView {
        let control = Group {
            if let url = URL(string: element.url) {
                Link(destination: url) {
                    ZStack {
                        RoundedRectangle(cornerRadius: 12)
                            .fill(Color.black.opacity(0.85))
                        VStack(spacing: 8) {
                            Image(systemName: "play.rectangle.fill")
                                .font(.system(size: 28))
                                .foregroundColor(.white)
                            Text("Open Video")
                                .foregroundColor(.white)
                        }
                    }
                }
                .buttonStyle(.plain)
            } else {
                Color.clear
            }
        }

        return positioned(control, element: element)
    }

    private func eventBlockerView(for element: EventBlockerElement) -> AnyView {
        positioned(EventBlockerPlatformView(), element: element)
    }

    public var body: some View {
        ZStack(alignment: .topLeading) {
            ForEach(sortedTextElements(), id: \.id) { element in
                textView(for: element)
            }
            ForEach(sortedElements(nativeImageElements.elements), id: \.id) { element in
                nativeImageView(for: element)
            }
            ForEach(sortedElements(youtubeVideoElements.elements), id: \.id) { element in
                youtubeVideoView(for: element)
            }
            ForEach(sortedElements(buttonElements.elements), id: \.id) { element in
                buttonView(for: element)
            }
            ForEach(sortedElements(checkboxElements.elements), id: \.id) { element in
                checkboxView(for: element)
            }
            ForEach(sortedElements(sliderElements.elements), id: \.id) { element in
                sliderView(for: element)
            }
            ForEach(sortedElements(dropdownElements.elements), id: \.id) { element in
                dropdownView(for: element)
            }
            ForEach(sortedElements(radioSetElements.elements), id: \.id) { element in
                radioSetView(for: element)
            }
            ForEach(sortedElements(textboxElements.elements), id: \.id) { element in
                textboxView(for: element)
            }
            ForEach(sortedElements(eventBlockerElements.elements), id: \.id) { element in
                eventBlockerView(for: element)
            }
        }
    }
}

private func configuredImage(_ image: Image, fit: String) -> some View {
    Group {
        switch fit {
        case "cover":
            image.resizable().scaledToFill()
        case "fill":
            image.resizable()
        default:
            image.resizable().scaledToFit()
        }
    }
    .clipped()
}

#if os(iOS) || os(tvOS) || os(watchOS)
private func loadPlatformImage(path: String) -> UIImage? {
    UIImage(contentsOfFile: path)
}

private func configuredPlatformImage(_ image: UIImage, fit: String) -> some View {
    configuredImage(Image(uiImage: image), fit: fit)
}
#elseif os(macOS)
private func loadPlatformImage(path: String) -> NSImage? {
    NSImage(contentsOfFile: path)
}

private func configuredPlatformImage(_ image: NSImage, fit: String) -> some View {
    configuredImage(Image(nsImage: image), fit: fit)
}
#endif

#if os(iOS) || os(tvOS) || os(watchOS)
private struct EventBlockerPlatformView: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isUserInteractionEnabled = true
        return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {}
}

private struct PaxTextboxField: UIViewRepresentable {
    let element: TextboxElement

    func makeCoordinator() -> Coordinator {
        Coordinator(id: element.id)
    }

    func makeUIView(context: Context) -> UITextField {
        let field = UITextField()
        field.borderStyle = .none
        field.autocorrectionType = .no
        field.autocapitalizationType = .none
        field.addTarget(context.coordinator, action: #selector(Coordinator.textDidChange(_:)), for: .editingChanged)
        field.delegate = context.coordinator
        return field
    }

    func updateUIView(_ uiView: UITextField, context: Context) {
        context.coordinator.id = element.id
        context.coordinator.isProgrammaticChange = true
        if uiView.text != element.text {
            uiView.text = element.text
        }
        context.coordinator.isProgrammaticChange = false

        uiView.placeholder = element.placeholder
        uiView.font = element.style.font.getUIFont(size: element.style.font_size)
        uiView.textColor = platformColor(element.style.fill)
        uiView.textAlignment = platformTextAlignment(element.style.alignmentMultiline)
        uiView.backgroundColor = platformColor(element.background)
        uiView.layer.cornerRadius = element.borderRadius
        uiView.layer.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        uiView.layer.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor

        if element.focusOnMount && !uiView.isFirstResponder {
            DispatchQueue.main.async {
                uiView.becomeFirstResponder()
            }
        }
    }

    final class Coordinator: NSObject, UITextFieldDelegate {
        var id: PaxNodeId
        var isProgrammaticChange = false

        init(id: PaxNodeId) {
            self.id = id
        }

        @objc func textDidChange(_ sender: UITextField) {
            guard !isProgrammaticChange else {
                return
            }
            dispatchFormTextboxInput(id: id, text: sender.text ?? "")
        }

        func textFieldDidEndEditing(_ textField: UITextField) {
            dispatchFormTextboxChange(id: id, text: textField.text ?? "")
        }
    }
}

private struct PaxTextboxArea: UIViewRepresentable {
    let element: TextboxElement

    func makeCoordinator() -> Coordinator {
        Coordinator(id: element.id)
    }

    func makeUIView(context: Context) -> UITextView {
        let view = UITextView()
        view.backgroundColor = .clear
        view.delegate = context.coordinator
        view.textContainerInset = UIEdgeInsets(top: 6, left: 4, bottom: 6, right: 4)
        return view
    }

    func updateUIView(_ uiView: UITextView, context: Context) {
        context.coordinator.id = element.id
        context.coordinator.isProgrammaticChange = true
        if uiView.text != element.text {
            uiView.text = element.text
        }
        context.coordinator.isProgrammaticChange = false

        uiView.font = element.style.font.getUIFont(size: element.style.font_size)
        uiView.textColor = platformColor(element.style.fill)
        uiView.backgroundColor = platformColor(element.background)
        uiView.layer.cornerRadius = element.borderRadius
        uiView.layer.borderWidth = CGFloat(max(element.outlineWidth, element.strokeWidth))
        uiView.layer.borderColor = platformColor(element.outlineWidth > 0 ? element.outlineColor : element.strokeColor).cgColor
        uiView.textAlignment = platformTextAlignment(element.style.alignmentMultiline)

        if element.focusOnMount && !uiView.isFirstResponder {
            DispatchQueue.main.async {
                uiView.becomeFirstResponder()
            }
        }
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var id: PaxNodeId
        var isProgrammaticChange = false

        init(id: PaxNodeId) {
            self.id = id
        }

        func textViewDidChange(_ textView: UITextView) {
            guard !isProgrammaticChange else {
                return
            }
            dispatchFormTextboxInput(id: id, text: textView.text)
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            dispatchFormTextboxChange(id: id, text: textView.text)
        }
    }
}

private struct EditableTextView: UIViewRepresentable {
    let element: TextElement

    func makeCoordinator() -> Coordinator {
        Coordinator(id: element.id)
    }

    func makeUIView(context: Context) -> UITextView {
        let view = UITextView()
        view.backgroundColor = .clear
        view.delegate = context.coordinator
        view.isScrollEnabled = false
        view.textContainerInset = .zero
        view.textContainer.lineFragmentPadding = 0
        return view
    }

    func updateUIView(_ uiView: UITextView, context: Context) {
        context.coordinator.id = element.id
        context.coordinator.isProgrammaticChange = true
        if uiView.text != element.content {
            uiView.text = element.content
        }
        context.coordinator.isProgrammaticChange = false

        uiView.font = element.textStyle.font.getUIFont(size: element.textStyle.font_size)
        uiView.textColor = platformColor(element.textStyle.fill)
        uiView.textAlignment = platformTextAlignment(element.textStyle.alignmentMultiline)
        uiView.isEditable = element.editable
        uiView.isSelectable = element.selectable || element.editable
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var id: PaxNodeId
        var isProgrammaticChange = false

        init(id: PaxNodeId) {
            self.id = id
        }

        func textViewDidChange(_ textView: UITextView) {
            guard !isProgrammaticChange else {
                return
            }
            dispatchTextInput(id: id, text: textView.text)
        }
    }
}
#else
private struct EventBlockerPlatformView: View {
    var body: some View {
        Color.clear
            .contentShape(Rectangle())
    }
}

private struct PaxTextboxField: View {
    let element: TextboxElement

    var body: some View {
        TextField(
            "",
            text: Binding(
                get: { element.text },
                set: {
                    dispatchFormTextboxInput(id: element.id, text: $0)
                    dispatchFormTextboxChange(id: element.id, text: $0)
                }
            ),
            prompt: element.placeholder.isEmpty ? nil : Text(element.placeholder)
        )
    }
}

private struct PaxTextboxArea: View {
    let element: TextboxElement

    var body: some View {
        TextEditor(
            text: Binding(
                get: { element.text },
                set: {
                    dispatchFormTextboxInput(id: element.id, text: $0)
                    dispatchFormTextboxChange(id: element.id, text: $0)
                }
            )
        )
    }
}

private struct EditableTextView: View {
    let element: TextElement

    var body: some View {
        TextEditor(
            text: Binding(
                get: { element.content },
                set: { dispatchTextInput(id: element.id, text: $0) }
            )
        )
    }
}
#endif

public class TextElements: ObservableObject {
    public static let singleton = TextElements()
    @Published public var elements: [PaxNodeId: TextElement] = [:]

    public func add(element: TextElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class FrameElements: ObservableObject {
    public static let singleton = FrameElements()
    @Published public var elements: [PaxNodeId: FrameElement] = [:]

    public func add(element: FrameElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }

    public func get(id: PaxNodeId) -> FrameElement? {
        elements[id]
    }
}

public class ButtonElements: ObservableObject {
    public static let singleton = ButtonElements()
    @Published public var elements: [PaxNodeId: ButtonElement] = [:]

    public func add(element: ButtonElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class CheckboxElements: ObservableObject {
    public static let singleton = CheckboxElements()
    @Published public var elements: [PaxNodeId: CheckboxElement] = [:]

    public func add(element: CheckboxElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class NativeImageElements: ObservableObject {
    public static let singleton = NativeImageElements()
    @Published public var elements: [PaxNodeId: NativeImageElement] = [:]

    public func add(element: NativeImageElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class YoutubeVideoElements: ObservableObject {
    public static let singleton = YoutubeVideoElements()
    @Published public var elements: [PaxNodeId: YoutubeVideoElement] = [:]

    public func add(element: YoutubeVideoElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class DropdownElements: ObservableObject {
    public static let singleton = DropdownElements()
    @Published public var elements: [PaxNodeId: DropdownElement] = [:]

    public func add(element: DropdownElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class RadioSetElements: ObservableObject {
    public static let singleton = RadioSetElements()
    @Published public var elements: [PaxNodeId: RadioSetElement] = [:]

    public func add(element: RadioSetElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class SliderElements: ObservableObject {
    public static let singleton = SliderElements()
    @Published public var elements: [PaxNodeId: SliderElement] = [:]

    public func add(element: SliderElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class TextboxElements: ObservableObject {
    public static let singleton = TextboxElements()
    @Published public var elements: [PaxNodeId: TextboxElement] = [:]

    public func add(element: TextboxElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}

public class EventBlockerElements: ObservableObject {
    public static let singleton = EventBlockerElements()
    @Published public var elements: [PaxNodeId: EventBlockerElement] = [:]

    public func add(element: EventBlockerElement) {
        elements[element.id] = element
    }

    public func remove(id: PaxNodeId) {
        elements.removeValue(forKey: id)
    }
}
