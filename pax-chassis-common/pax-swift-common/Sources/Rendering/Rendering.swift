import SwiftUI
import Messages

public struct NativeRenderingLayer: View {
    
    public init() {}

    @ObservedObject var textElements : TextElements = TextElements.singleton
    @ObservedObject var frameElements : FrameElements = FrameElements.singleton

    public func getClippingMask(clippingIds: [[UInt64]]) -> some View {

        var elements : [FrameElement] = []

        clippingIds.makeIterator().forEach( { id_chain in
            elements.insert(self.frameElements.elements[id_chain]!, at: 0)
        })

        return ZStack { ForEach(elements, id: \.id_chain) { frameElement in
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

    @ViewBuilder
    public func getPositionedTextGroup(textElement: TextElement) -> some View {
        let transform = CGAffineTransform.init(
                a: CGFloat(textElement.transform[0]),
                b: CGFloat(textElement.transform[1]),
                c: CGFloat(textElement.transform[2]),
                d: CGFloat(textElement.transform[3]),
                tx: CGFloat(textElement.transform[4]),
                ty: CGFloat(textElement.transform[5])
        )
        var text: AttributedString {
            var attributedString: AttributedString = try! AttributedString(markdown: textElement.content, options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace))

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
        let textView : some View =
                Text(text)
                        .foregroundColor(textElement.textStyle.fill)
                        .font(textElement.textStyle.font.getFont(size: textElement.textStyle.font_size))
                        .frame(width: CGFloat(textElement.size_x), height: CGFloat(textElement.size_y), alignment: textElement.textStyle.alignment)
                        .position(x: CGFloat(textElement.size_x / 2.0), y: CGFloat(textElement.size_y / 2.0))
                        .transformEffect(transform)
                        .textSelection(.enabled)

//
//            if !textElement.clipping_ids.isEmpty {
//                textView.mask(getClippingMask(clippingIds: textElement.clipping_ids))
//            } else {
//                textView
//            }

        textView
    }

    public var body: some View {
        ZStack{
            ForEach(Array(self.textElements.elements.values), id: \.id_chain) { textElement in
                getPositionedTextGroup(textElement: textElement)
            }
        }
    }
}

public class TextElements: ObservableObject {
    public static let singleton : TextElements = TextElements()

    @Published public var elements : [[UInt64]: TextElement] = [:]

    public func add(element: TextElement) {
        self.elements[element.id_chain] = element
    }
    public func remove(id: [UInt64]) {
        self.elements.removeValue(forKey: id)
    }
}

public class FrameElements: ObservableObject {
    public static let singleton : FrameElements = FrameElements()

    @Published public var elements : [[UInt64]: FrameElement] = [:]

    public func add(element: FrameElement) {
        self.elements[element.id_chain] = element
    }
    public func remove(id: [UInt64]) {
        self.elements.removeValue(forKey: id)
    }
    public func get(id: [UInt64]) -> FrameElement? {
        return self.elements[id]
    }
}
