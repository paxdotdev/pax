import SwiftUI
import Messages

public struct NativeRenderingLayer: View {
    
    public init() {}

    @ObservedObject var textElements : TextElements = TextElements.singleton
    @ObservedObject var frameElements : FrameElements = FrameElements.singleton

    private func getClippingFrames(startingAt parentFrame: PaxNodeId?) -> [FrameElement] {
        var elements: [FrameElement] = []
        var currentFrame = parentFrame

        while let frameId = currentFrame, let frame = self.frameElements.elements[frameId] {
            if frame.clipContent {
                elements.insert(frame, at: 0)
            }
            currentFrame = frame.parentFrame
        }

        return elements
    }

    public func getClippingMask(parentFrame: PaxNodeId?) -> some View {
        let elements = getClippingFrames(startingAt: parentFrame)

        return ZStack { ForEach(elements, id: \.id) { frameElement in
            Rectangle()
                    .frame(width: CGFloat(frameElement.size_x), height: CGFloat(frameElement.size_y))
                    .position(x: CGFloat(frameElement.size_x / 2.0), y: CGFloat(frameElement.size_y / 2.0))
                    .transformEffect(CGAffineTransform.init(
                            a: CGFloat(frameElement.transform[0]),
                            b: CGFloat(frameElement.transform[1]),
                            c: CGFloat(frameElement.transform[2]),
                            d: CGFloat(frameElement.transform[3]),
                            tx: CGFloat(frameElement.transform[4]),
                            ty: CGFloat(frameElement.transform[5]))
                    )
        } }
    }

    public func getPositionedTextGroup(textElement: TextElement) -> AnyView {
        let transform = CGAffineTransform.init(
                a: CGFloat(textElement.transform[0]),
                b: CGFloat(textElement.transform[1]),
                c: CGFloat(textElement.transform[2]),
                d: CGFloat(textElement.transform[3]),
                tx: CGFloat(textElement.transform[4]),
                ty: CGFloat(textElement.transform[5])
        )
        let measuredWidth = textElement.size_x >= 0 ? CGFloat(textElement.size_x) : textElement.lastMeasuredSize?.width
        let measuredHeight = textElement.size_y >= 0 ? CGFloat(textElement.size_y) : textElement.lastMeasuredSize?.height
        let positionWidth = measuredWidth ?? 0
        let positionHeight = measuredHeight ?? 0
        var text: AttributedString {
            var attributedString: AttributedString
            if textElement.markdown {
                attributedString = try! AttributedString(markdown: textElement.content, options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace))
            } else {
                attributedString = AttributedString(textElement.content)
            }

            for run in attributedString.runs {
                if run.link != nil {
                    if let linkStyle = textElement.style_link {
                        attributedString[run.range].font = linkStyle.font.getFont(size: linkStyle.font_size)
                        if(linkStyle.underline){
                            attributedString[run.range].underlineStyle = .single
                        } else {
                            attributedString[run.range].underlineStyle = .none
                        }
                        attributedString[run.range].foregroundColor = linkStyle.fill
                    }
                }
            }
            return attributedString

        }
        let clippingFrames = getClippingFrames(startingAt: textElement.parentFrame)

        let selectedTextView: AnyView
        if textElement.selectable {
            selectedTextView = AnyView(
                Text(text)
                    .foregroundColor(textElement.textStyle.fill)
                    .font(textElement.textStyle.font.getFont(size: textElement.textStyle.font_size))
                    .frame(width: measuredWidth, height: measuredHeight, alignment: textElement.textStyle.alignment)
                    .position(x: positionWidth / 2.0, y: positionHeight / 2.0)
                    .transformEffect(transform)
                    .textSelection(.enabled)
                    .allowsHitTesting(true)
                    .zIndex(Double(textElement.zIndex))
            )
        } else {
            selectedTextView = AnyView(
                Text(text)
                    .foregroundColor(textElement.textStyle.fill)
                    .font(textElement.textStyle.font.getFont(size: textElement.textStyle.font_size))
                    .frame(width: measuredWidth, height: measuredHeight, alignment: textElement.textStyle.alignment)
                    .position(x: positionWidth / 2.0, y: positionHeight / 2.0)
                    .transformEffect(transform)
                    .textSelection(.disabled)
                    .allowsHitTesting(false)
                    .zIndex(Double(textElement.zIndex))
            )
        }

        if clippingFrames.isEmpty {
            return selectedTextView
        }
        return AnyView(selectedTextView.mask(getClippingMask(parentFrame: textElement.parentFrame)))
    }

    public var body: some View {
        ZStack{
            ForEach(Array(self.textElements.elements.values).sorted(by: { lhs, rhs in
                if lhs.zIndex == rhs.zIndex {
                    return lhs.id < rhs.id
                }
                return lhs.zIndex < rhs.zIndex
            }), id: \.id) { textElement in
                getPositionedTextGroup(textElement: textElement)
            }
        }
    }
}

public class TextElements: ObservableObject {
    public static let singleton : TextElements = TextElements()

    @Published public var elements : [PaxNodeId: TextElement] = [:]

    public func add(element: TextElement) {
        self.elements[element.id] = element
    }
    public func remove(id: PaxNodeId) {
        self.elements.removeValue(forKey: id)
    }
}

public class FrameElements: ObservableObject {
    public static let singleton : FrameElements = FrameElements()

    @Published public var elements : [PaxNodeId: FrameElement] = [:]

    public func add(element: FrameElement) {
        self.elements[element.id] = element
    }
    public func remove(id: PaxNodeId) {
        self.elements.removeValue(forKey: id)
    }
    public func get(id: PaxNodeId) -> FrameElement? {
        return self.elements[id]
    }
}
