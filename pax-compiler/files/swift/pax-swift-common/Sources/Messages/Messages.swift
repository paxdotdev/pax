//
//  Messages.swift
//  interface
//
//  Created by Zachary Brown on 5/7/22.
//

import Foundation
import SwiftUI
import FlexBuffers
#if os(iOS) || os(tvOS) || os(watchOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

public typealias PaxNodeId = UInt32

public final class NativeInterruptDispatcher {
    public static let shared = NativeInterruptDispatcher()

    public var sendData: ((Data) -> Void)?

    private init() {}

    public func send(_ data: Data) {
        sendData?(data)
    }
}

private func dispatchNativeInterrupt(_ build: (FlexBufferMapBuilder) throws -> Void) {
    let buffer = try! FlexBufferBuilder.encodeMap(build)
    NativeInterruptDispatcher.shared.send(buffer.data)
}

public func dispatchChassisResizeRequest(id: PaxNodeId, width: Double, height: Double) {
    dispatchNativeInterrupt { builder in
        builder.addVectorWithStringKey("ChassisResizeRequestCollection") { vectorBuilder in
            vectorBuilder.addMap { requestBuilder in
                requestBuilder.addWithStringKey("id", UInt(id))
                requestBuilder.addWithStringKey("width", width)
                requestBuilder.addWithStringKey("height", height)
            }
        }
    }
}

public func dispatchFormButtonClick(id: PaxNodeId) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormButtonClick") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
        }
    }
}

public func dispatchFormCheckboxToggle(id: PaxNodeId, state: Bool) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormCheckboxToggle") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("state", state)
        }
    }
}

public func dispatchFormDropdownChange(id: PaxNodeId, selectedId: UInt32) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormDropdownChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("selected_id", UInt(selectedId))
        }
    }
}

public func dispatchFormRadioSetChange(id: PaxNodeId, selectedId: UInt32) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormRadioSetChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("selected_id", UInt(selectedId))
        }
    }
}

public func dispatchFormSliderChange(id: PaxNodeId, value: Double) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormSliderChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("value", value)
        }
    }
}

public func dispatchFormTextboxInput(id: PaxNodeId, text: String) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormTextboxInput") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addStringWithStringKey("text", text)
        }
    }
}

public func dispatchFormTextboxChange(id: PaxNodeId, text: String) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormTextboxChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addStringWithStringKey("text", text)
        }
    }
}

public func dispatchTextInput(id: PaxNodeId, text: String) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("TextInput") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addStringWithStringKey("text", text)
        }
    }
}

private func readNodeId(_ fb: FlxbReference?) -> PaxNodeId? {
    guard let fb, !fb.isNull else {
        return nil
    }
    if let value = fb.asUInt64 {
        return PaxNodeId(truncatingIfNeeded: value)
    }
    if let value = fb.asUInt {
        return PaxNodeId(truncatingIfNeeded: value)
    }
    return nil
}

private func readFloatArray(_ fb: FlxbReference?) -> [Float]? {
    guard let vector = fb?.asVector else {
        return nil
    }
    return vector.makeIterator().map { item in
        if let value = item.asFloat {
            return value
        }
        if let value = item.asDouble {
            return Float(value)
        }
        return 0.0
    }
}

private func readStringArray(_ fb: FlxbReference?) -> [String]? {
    guard let vector = fb?.asVector else {
        return nil
    }
    return vector.makeIterator().compactMap { $0.asString }
}

private func readDouble(_ fb: FlxbReference?) -> Double? {
    if let value = fb?.asDouble {
        return value
    }
    if let value = fb?.asFloat {
        return Double(value)
    }
    return nil
}

/// Agnostic of the type of element, this patch contains only create-time metadata.
public class AnyCreatePatch {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    
    public init(fb:FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.occlusionLayerId = UInt32(truncatingIfNeeded: fb["occlusion_layer_id"]?.asUInt64 ?? 0)
    }
}


public class AnyDeletePatch {
    public var id: PaxNodeId
    
    public init(fb:FlxbReference) {
        self.id = readNodeId(fb) ?? 0
    }
}

public class TextStyle {
    public var font: PaxFont
    public var fill: Color
    public var alignmentMultiline: TextAlignment
    public var alignment: Alignment
    public var font_size: CGFloat
    public var underline: Bool
    
    public init(font: PaxFont, fill: Color, alignmentMultiline: TextAlignment, alignment: Alignment, font_size: CGFloat, underline: Bool) {
        self.font = font
        self.fill = fill
        self.alignmentMultiline = alignmentMultiline
        self.alignment = alignment
        self.font_size = font_size
        self.underline = underline
    }
    
    public func applyPatch(from patch: TextStyleMessage) {
        
        self.font.applyPatch(fb: patch.font)

        if patch.fill != nil {
            self.fill = patch.fill!
        }
        
        if patch.align_multiline != nil {
            self.alignmentMultiline = patch.align_multiline!.toTextAlignment()
        } else if patch.align_horizontal != nil {
            self.alignmentMultiline = patch.align_horizontal!.toTextAlignment()
        }
        if patch.align_vertical != nil && patch.align_horizontal != nil {
            self.alignment = toAlignment(horizontalAlignment: patch.align_horizontal!, verticalAlignment: patch.align_vertical!)
        }
        
        if patch.font_size != nil {
            self.font_size = patch.font_size!
        }
        
        if patch.underline != nil {
            self.underline = patch.underline!
        }
    }
}

public class TextElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var content: String
    public var editable: Bool
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var textStyle: TextStyle
    public var selectable: Bool
    public var markdown: Bool
    public var style_link: TextStyle?
    public var lastMeasuredSize: CGSize?
    
    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, content: String, editable: Bool, transform: [Float], size_x: Float, size_y: Float, textStyle: TextStyle, selectable: Bool, markdown: Bool, style_link: TextStyle?, lastMeasuredSize: CGSize? = nil) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.content = content
        self.editable = editable
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.textStyle = textStyle
        self.selectable = selectable
        self.markdown = markdown
        self.style_link = style_link
        self.lastMeasuredSize = lastMeasuredSize
    }
    
    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> TextElement {
        let defaultTextStyle = TextStyle(font: PaxFont.makeDefault(), fill: Color(.black), alignmentMultiline: .leading, alignment: .topLeading, font_size: 5.0, underline: false)
        return TextElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, content: "", editable: false, transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0, textStyle: defaultTextStyle, selectable: false, markdown: false, style_link: nil)
    }
    
    public func applyPatch(patch: TextUpdatePatch) {
        //no-op to ID, as it is primary key
        
        if let content = patch.content {
            self.content = content
        }
        if let editable = patch.editable {
            self.editable = editable
        }
        if let transform = patch.transform {
            self.transform = transform
        }
        if let size_x = patch.size_x {
            self.size_x = size_x
        }
        if let size_y = patch.size_y {
            self.size_y = size_y
        }
        if let selectable = patch.selectable {
            self.selectable = selectable
        }
        if let markdown = patch.markdown {
            self.markdown = markdown
        }
        
        // Apply new TextStyle
        if let styleBuffer = patch.style {
            self.textStyle.applyPatch(from: styleBuffer)
        }
        
        // Apply style_link
        if let styleLinkBuffer = patch.style_link {
            if self.style_link == nil {
                self.style_link = TextStyle(
                    font: PaxFont.makeDefault(),
                    fill: self.textStyle.fill,
                    alignmentMultiline: self.textStyle.alignmentMultiline,
                    alignment: self.textStyle.alignment,
                    font_size: self.textStyle.font_size,
                    underline: self.textStyle.underline
                )
            }
            self.style_link?.applyPatch(from: styleLinkBuffer)
        }
    }

    public func applyOcclusionPatch(_ patch: OcclusionUpdatePatch) {
        self.parentFrame = patch.parentFrame
        self.occlusionLayerId = patch.occlusionLayerId
        self.zIndex = patch.zIndex
    }
}

public enum TextAlignHorizontal {
    case center
    case left
    case right
}

public extension TextAlignHorizontal {
    func toTextAlignment() -> TextAlignment {
        switch self {
        case .center:
            return .center
        case .left:
            return .leading
        case .right:
            return .trailing
        }
    }
}

public enum TextAlignVertical {
    case top
    case center
    case bottom
}


public func toAlignment(horizontalAlignment: TextAlignHorizontal, verticalAlignment: TextAlignVertical) -> Alignment {
    let horizontal: HorizontalAlignment
    let vertical: VerticalAlignment
    
    switch horizontalAlignment {
    case .center:
        horizontal = .center
    case .left:
        horizontal = .leading
    case .right:
        horizontal = .trailing
    }
    
    switch verticalAlignment {
    case .top:
        vertical = .top
    case .center:
        vertical = .center
    case .bottom:
        vertical = .bottom
    }
    return Alignment(horizontal: horizontal, vertical: vertical)
}


/// A patch representing an image load request from a given id_chain
public class ImageLoadPatch {
    public var id: PaxNodeId
    public var path: String?
    
    public init(fb:FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.path = fb["path"]?.asString
    }
}


public class TextStyleMessage {
    public var font: FlxbReference
    public var fill: Color?
    public var font_size: CGFloat?
    public var underline: Bool?
    public var align_multiline: TextAlignHorizontal?
    public var align_horizontal: TextAlignHorizontal?
    public var align_vertical: TextAlignVertical?
    
    public init(_ buffer: FlxbReference) {
        self.font =  buffer["font"]!
        
        self.font_size = buffer["font_size"]?.asFloat.map { CGFloat($0) }
        self.underline = buffer["underline"]?.asBool
        
        if let alignmentValue = buffer["align_multiline"]?.asString {
            switch alignmentValue {
            case "Center":
                self.align_multiline = .center
            case "Left":
                self.align_multiline = .left
            case "Right":
                self.align_multiline = .right
            default:
                self.align_multiline = nil
            }
        }
        
        if let alignmentValue = buffer["align_horizontal"]?.asString {
            switch alignmentValue {
            case "Center":
                self.align_horizontal = .center
            case "Left":
                self.align_horizontal = .left
            case "Right":
                self.align_horizontal = .right
            default:
                self.align_horizontal = nil
            }
        }
        
        if let verticalAlignmentValue = buffer["align_vertical"]?.asString {
            switch verticalAlignmentValue {
            case "Top":
                self.align_vertical = .top
            case "Center":
                self.align_vertical = .center
            case "Bottom":
                self.align_vertical = .bottom
            default:
                self.align_vertical = nil
            }
        }
        
        if let colorBuffer = buffer["fill"], !colorBuffer.isNull {
            self.fill = extractColorFromBuffer(colorBuffer)
        }
    }
}


public class TextUpdatePatch {
    public var id: PaxNodeId
    public var content: String?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var editable: Bool?
    public var selectable: Bool?
    public var markdown: Bool?
    public var style: TextStyleMessage?
    public var style_link: TextStyleMessage?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.content = fb["content"]?.asString
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.editable = fb["editable"]?.asBool
        self.selectable = fb["selectable"]?.asBool
        self.markdown = fb["markdown"]?.asBool
        
        if let styleBuffer = fb["style"], !styleBuffer.isNull {
            self.style = TextStyleMessage(styleBuffer)
        }
        
        if let styleLinkBuffer = fb["style_link"], !styleLinkBuffer.isNull {
            self.style_link = TextStyleMessage(styleLinkBuffer)
        }
    }
}

///// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
//public class TextUpdatePatch {
//    public var id_chain: [UInt64]
//    public var content: String?
//    public var transform: [Float]?
//    public var size_x: Float?
//    public var size_y: Float?
//    public var fontBuffer: FlxbReference
//    public var fill: Color?
//    public var align_multiline: TextAlignHorizontal?
//    public var align_vertical: TextAlignVertical?
//    public var align_horizontal: TextAlignHorizontal?
//    // New properties
//    public var size: CGFloat?
//    public var style_link: LinkStyle?
//
//    init(fb: FlxbReference) {
//        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
//            fb.asUInt64!
//        })
//        self.content = fb["content"]?.asString
//        self.transform = fb["transform"]?.asVector?.makeIterator().map({ fb in
//            fb.asFloat!
//        })
//        self.size_x = fb["size_x"]?.asFloat
//        self.size_y = fb["size_y"]?.asFloat
//        self.fontBuffer =  fb["font"]!
//
//        if let fillBuffer = fb["fill"], !fillBuffer.isNull {
//            self.fill = extractColorFromBuffer(fillBuffer)
//        }
//
//        if let alignmentValue = fb["align_multiline"]?.asString {
//            switch alignmentValue {
//            case "Center":
//                self.align_multiline = .center
//            case "Left":
//                self.align_multiline = .left
//            case "Right":
//                self.align_multiline = .right
//            default:
//                self.align_multiline = nil
//            }
//        }
//
//        if let verticalAlignmentValue = fb["align_vertical"]?.asString {
//            switch verticalAlignmentValue {
//            case "Top":
//                self.align_vertical = .top
//            case "Center":
//                self.align_vertical = .center
//            case "Bottom":
//                self.align_vertical = .bottom
//            default:
//                self.align_vertical = nil
//            }
//        }
//
//        if let alignmentValue = fb["align_horizontal"]?.asString {
//            switch alignmentValue {
//            case "Center":
//                self.align_horizontal = .center
//            case "Left":
//                self.align_horizontal = .left
//            case "Right":
//                self.align_horizontal = .right
//            default:
//                self.align_horizontal = nil
//            }
//        }
//
//        self.size = fb["size"]?.asFloat.map { CGFloat($0) }
//
//        if !fb["style_link"]!.isNull {
//            self.style_link = LinkStyle(fb: fb["style_link"]!)
//        }
//
//    }
//}


public func extractColorFromBuffer(_ fillBuffer: FlxbReference) -> Color {
    if let rgba = fillBuffer["Rgba"], !rgba.isNull {
        let stub = fillBuffer["Rgba"]!
        return Color(
            red: readDouble(stub[0]) ?? 0.0,
            green: readDouble(stub[1]) ?? 0.0,
            blue: readDouble(stub[2]) ?? 0.0,
            opacity: readDouble(stub[3]) ?? 1.0
        )
    } else if let rgb = fillBuffer["Rgb"], !rgb.isNull {
        let stub = fillBuffer["Rgb"]!
        return Color(
            red: readDouble(stub[0]) ?? 0.0,
            green: readDouble(stub[1]) ?? 0.0,
            blue: readDouble(stub[2]) ?? 0.0,
            opacity: 1.0
        )
    } else {
        return Color.black
    }
}

public enum TextAlignHorizontalMessage: String {
    case Left, Center, Right
}

public enum FontStyle: String {
    case normal = "Normal"
    case italic = "Italic"
    case oblique = "Oblique"
}

extension FontWeight {
    public func fontWeight() -> Font.Weight {
        switch self {
        case .thin: return .thin
        case .extraLight: return .ultraLight
        case .light: return .light
        case .normal: return .regular
        case .medium: return .medium
        case .semiBold: return .semibold
        case .bold: return .bold
        case .extraBold: return .heavy
        case .black: return .black
        }
    }
}

#if os(iOS) || os(tvOS) || os(watchOS)
extension FontWeight {
    public func uiFontWeight() -> UIFont.Weight {
        switch self {
        case .thin: return .thin
        case .extraLight: return .ultraLight
        case .light: return .light
        case .normal: return .regular
        case .medium: return .medium
        case .semiBold: return .semibold
        case .bold: return .bold
        case .extraBold: return .heavy
        case .black: return .black
        }
    }
}
#elseif os(macOS)
extension FontWeight {
    public func nsFontWeight() -> NSFont.Weight {
        switch self {
        case .thin: return .thin
        case .extraLight: return .ultraLight
        case .light: return .light
        case .normal: return .regular
        case .medium: return .medium
        case .semiBold: return .semibold
        case .bold: return .bold
        case .extraBold: return .heavy
        case .black: return .black
        }
    }
}
#endif

public enum FontWeight: String {
    case thin = "Thin"
    case extraLight = "ExtraLight"
    case light = "Light"
    case normal = "Normal"
    case medium = "Medium"
    case semiBold = "SemiBold"
    case bold = "Bold"
    case extraBold = "ExtraBold"
    case black = "Black"
}

public class PaxFont {
    public enum PaxFontType {
        case system(SystemFont)
        case web(WebFont)
        case local(LocalFont)
    }

    public struct SystemFont {
        let family: String
        let style: FontStyle
        let weight: FontWeight
    }

    public struct WebFont {
        let family: String
        let url: URL
        let style: FontStyle
        let weight: FontWeight
    }

    public struct LocalFont {
        let family: String
        let path: URL
        let style: FontStyle
        let weight: FontWeight
    }

    public var type: PaxFontType
    public var cachedFont: Font?
    public var currentSize: CGFloat

    public init(type: PaxFontType) {
        self.type = type
        self.currentSize = 12
    }
    
    public static func makeDefault() -> PaxFont {
        let defaultSystemFont = SystemFont(family: "Helvetica", style: .normal, weight: .normal)
        return PaxFont(type: .system(defaultSystemFont))
    }
    
    public func getFont(size: CGFloat) -> Font {
        if let cachedFont = cachedFont, currentSize == size {
            return cachedFont
        }
        
        var fontFamily: String?
        var fontStyle: FontStyle?
        var fontWeight: FontWeight?

        switch type {
        case .system(let systemFont):
            fontFamily = systemFont.family
            fontStyle = systemFont.style
            fontWeight = systemFont.weight
        case .web(let webFont):
            fontFamily = webFont.family
            fontStyle = webFont.style
            fontWeight = webFont.weight
        case .local(let localFont):
            fontFamily = localFont.family
            fontStyle = localFont.style
            fontWeight = localFont.weight
        }
        
        let isFontRegistered = PaxFont.isFontRegistered(fontFamily: fontFamily!)
        
        let baseFont: Font
        if isFontRegistered {
            baseFont = Font.custom(fontFamily!, size: size).weight(fontWeight!.fontWeight())
        } else {
            baseFont = .system(size: size).weight(fontWeight!.fontWeight())
        }

        let finalFont: Font
        switch fontStyle! {
        case .normal:
            finalFont = baseFont
        case .italic:
            finalFont = baseFont.italic()
        case .oblique:
            finalFont = baseFont
        }

        cachedFont = finalFont
        currentSize = size

        return finalFont
    }

    #if os(iOS) || os(tvOS) || os(watchOS)
    public func getUIFont(size: CGFloat) -> UIFont {
        var fontFamily: String?
        var fontStyle: FontStyle?
        var fontWeight: FontWeight?

        switch type {
        case .system(let systemFont):
            fontFamily = systemFont.family
            fontStyle = systemFont.style
            fontWeight = systemFont.weight
        case .web(let webFont):
            fontFamily = webFont.family
            fontStyle = webFont.style
            fontWeight = webFont.weight
        case .local(let localFont):
            fontFamily = localFont.family
            fontStyle = localFont.style
            fontWeight = localFont.weight
        }

        let baseFont: UIFont
        if let fontFamily, PaxFont.isFontRegistered(fontFamily: fontFamily) {
            baseFont = UIFont(name: fontFamily, size: size) ?? UIFont.systemFont(ofSize: size, weight: fontWeight?.uiFontWeight() ?? .regular)
        } else {
            baseFont = UIFont.systemFont(ofSize: size, weight: fontWeight?.uiFontWeight() ?? .regular)
        }

        switch fontStyle ?? .normal {
        case .normal:
            return baseFont
        case .italic:
            if let descriptor = baseFont.fontDescriptor.withSymbolicTraits(.traitItalic) {
                return UIFont(descriptor: descriptor, size: size)
            }
            return UIFont.italicSystemFont(ofSize: size)
        case .oblique:
            return baseFont
        }
    }
    #elseif os(macOS)
    public func getNSFont(size: CGFloat) -> NSFont {
        var fontFamily: String?
        var fontStyle: FontStyle?
        var fontWeight: FontWeight?

        switch type {
        case .system(let systemFont):
            fontFamily = systemFont.family
            fontStyle = systemFont.style
            fontWeight = systemFont.weight
        case .web(let webFont):
            fontFamily = webFont.family
            fontStyle = webFont.style
            fontWeight = webFont.weight
        case .local(let localFont):
            fontFamily = localFont.family
            fontStyle = localFont.style
            fontWeight = localFont.weight
        }

        let baseFont: NSFont
        if let fontFamily, PaxFont.isFontRegistered(fontFamily: fontFamily) {
            baseFont = NSFont(name: fontFamily, size: size) ?? NSFont.systemFont(ofSize: size, weight: fontWeight?.nsFontWeight() ?? .regular)
        } else {
            baseFont = NSFont.systemFont(ofSize: size, weight: fontWeight?.nsFontWeight() ?? .regular)
        }

        switch fontStyle ?? .normal {
        case .normal:
            return baseFont
        case .italic:
            let descriptor = baseFont.fontDescriptor.withSymbolicTraits(.italic)
            return NSFont(descriptor: descriptor, size: size) ?? NSFontManager.shared.convert(baseFont, toHaveTrait: .italicFontMask)
        case .oblique:
            return baseFont
        }
    }
    #endif



    public func applyPatch(fb: FlxbReference) {
        if let systemFontMessage = fb["System"] {
            if let family = systemFontMessage["family"]?.asString {
                let styleMessage = FontStyle(rawValue: systemFontMessage["style"]?.asString ?? "normal") ?? .normal
                let weightMessage = FontWeight(rawValue: systemFontMessage["weight"]?.asString ?? "normal") ?? .normal
                self.type = .system(SystemFont(family: family, style: styleMessage, weight: weightMessage))
            }
        } else if let webFontMessage = fb["Web"] {
            if let family = webFontMessage["family"]?.asString,
               let urlString = webFontMessage["url"]?.asString,
               let url = URL(string: urlString) {
                let style = FontStyle(rawValue: webFontMessage["style"]?.asString ?? "normal") ?? .normal
                let weight = FontWeight(rawValue: webFontMessage["weight"]?.asString ?? "normal") ?? .normal

                self.type = .web(WebFont(family: family, url: url, style: style, weight: weight))
            }
        } else if let localFontMessage = fb["Local"] {
            if let family = localFontMessage["family"]?.asString,
               let pathString = localFontMessage["path"]?.asString,
               let path = URL(string: pathString) {
                let style = FontStyle(rawValue: localFontMessage["style"]?.asString ?? "normal") ?? .normal
                let weight = FontWeight(rawValue: localFontMessage["weight"]?.asString ?? "normal") ?? .normal

                self.type = .local(LocalFont(family: family, path: path, style: style, weight: weight))
            }
        }
    }

    #if os(macOS)
    public static func isFontRegistered(fontFamily: String) -> Bool {
        let fontFamilies = CTFontManagerCopyAvailableFontFamilyNames() as! [String]

        if fontFamilies.contains(fontFamily) {
            return true
        }

        // Check if the font is installed on the system using CTFontManager
        let installedFontURLs = CTFontManagerCopyAvailableFontURLs() as? [URL] ?? []

        for url in installedFontURLs {
            if let fontDescriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] {
                for descriptor in fontDescriptors {
                    if let fontFamilyName = CTFontDescriptorCopyAttribute(descriptor, kCTFontFamilyNameAttribute) as? String {
                        if fontFamilyName == fontFamily {
                            return true
                        }
                    }
                }
            }
        }
        return false

    }
    #elseif  os(iOS) || os(tvOS) || os(watchOS)
    public static func isFontRegistered(fontFamily: String) -> Bool {
        let availableFontFamilies = UIFont.familyNames
        
        return availableFontFamilies.contains(fontFamily)
    }
    #endif
}
//
//public class FontFactory {
////    public var family: String
////    public var public variant: String
////    public var size: Float
//
//    func applyPatch(fb: FlxbReference) -> Font {
//        print("MAKING FONT")
//        print(fb.debugDescription)
//
//
//
//        var suffix = ""
//        if fb["variant"] != nil && !fb["variant"]!.isNull { && fb["variant"]!.asString! != "Regular" {
//            suffix = " " + fb["variant"]!.asString!
//        }
//        return Font.custom(String(fb["family"]!.asString! + suffix), size: CGFloat(fb["size"]!.asFloat!))
//    }
//
//
//    static func makeDefault() -> Font {
//        return Font.custom("Courier New", size: 14)
////        Font()
////        return Font(family: "Courier New", variant: "Regular", size: 14)
//    }
//}

//func registerWebFont() {
//       if case let .web(webFont) = type, !Self.isFontRegistered(fontFamily: webFont.family) {
//           URLSession.shared.dataTask(with: webFont.url) { data, response, error in
//               guard let data = data, error == nil else {
//                   print("Error downloading font: \(String(describing: error))")
//                   return
//               }
//               guard let provider = CGDataProvider(data: data as CFData) else {
//                   print("Error creating font provider")
//                   return
//               }
//               guard let font = CGFont(provider) else {
//                   print("Error creating font from data")
//                   return
//               }
//               print(font.fullName)
//               var errorRef: Unmanaged<CFError>?
//               if !CTFontManagerRegisterGraphicsFont(font, &errorRef) {
//                   print("Error registering font: \(webFont.family) - \(String(describing: errorRef))")
//               }
//           }.resume()
//       }
//   }



public class FrameElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var clipContent: Bool
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    
    public init(id: PaxNodeId, parentFrame: PaxNodeId?, clipContent: Bool, zIndex: Int, transform: [Float], size_x: Float, size_y: Float) {
        self.id = id
        self.parentFrame = parentFrame
        self.clipContent = clipContent
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
    }
    
    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?) -> FrameElement {
        FrameElement(id: id, parentFrame: parentFrame, clipContent: false, zIndex: 0, transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0)
    }
    
    public func applyPatch(patch: FrameUpdatePatch) {
        //no-op to ID, as it is primary key
        
        if patch.transform != nil {
            self.transform = patch.transform!
        }
        if patch.size_x != nil {
            self.size_x = patch.size_x!
        }
        if patch.size_y != nil {
            self.size_y = patch.size_y!
        }
        if let clipContent = patch.clipContent {
            self.clipContent = clipContent
        }
    }

    public func applyOcclusionPatch(_ patch: OcclusionUpdatePatch) {
        self.parentFrame = patch.parentFrame
        self.zIndex = patch.zIndex
    }
}



/// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
public class FrameUpdatePatch {
    public var id: PaxNodeId
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var clipContent: Bool?
    
    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.clipContent = fb["clip_content"]?.asBool
    }
}

public class OcclusionUpdatePatch {
    public var id: PaxNodeId
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var parentFrame: PaxNodeId?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.occlusionLayerId = UInt32(truncatingIfNeeded: fb["occlusion_layer_id"]?.asUInt64 ?? 0)
        self.zIndex = Int(fb["z_index"]?.asInt ?? 0)
        self.parentFrame = readNodeId(fb["parent_frame"])
    }
}

public func defaultPaxTextStyle() -> TextStyle {
    TextStyle(
        font: PaxFont.makeDefault(),
        fill: Color(.black),
        alignmentMultiline: .leading,
        alignment: .topLeading,
        font_size: 14.0,
        underline: false
    )
}

public protocol NativePositionElement: AnyObject {
    var id: PaxNodeId { get }
    var parentFrame: PaxNodeId? { get set }
    var occlusionLayerId: UInt32 { get set }
    var zIndex: Int { get set }
    var transform: [Float] { get set }
    var size_x: Float { get set }
    var size_y: Float { get set }
}

public extension NativePositionElement {
    func applyOcclusionPatch(_ patch: OcclusionUpdatePatch) {
        self.parentFrame = patch.parentFrame
        self.occlusionLayerId = patch.occlusionLayerId
        self.zIndex = patch.zIndex
    }
}

public class EventBlockerPatchMessage {
    public var id: PaxNodeId
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
    }
}

public class EventBlockerElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> EventBlockerElement {
        EventBlockerElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0)
    }

    public func applyPatch(_ patch: EventBlockerPatchMessage) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
    }
}

public class ButtonUpdatePatch {
    public var id: PaxNodeId
    public var hoverColor: Color?
    public var outlineStrokeColor: Color?
    public var outlineStrokeWidth: Double?
    public var borderRadius: Double?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var content: String?
    public var color: Color?
    public var style: TextStyleMessage?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.outlineStrokeWidth = readDouble(fb["outline_stroke_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.content = fb["content"]?.asString
        if let hoverColor = fb["hover_color"], !hoverColor.isNull {
            self.hoverColor = extractColorFromBuffer(hoverColor)
        }
        if let outlineStrokeColor = fb["outline_stroke_color"], !outlineStrokeColor.isNull {
            self.outlineStrokeColor = extractColorFromBuffer(outlineStrokeColor)
        }
        if let color = fb["color"], !color.isNull {
            self.color = extractColorFromBuffer(color)
        }
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
    }
}

public class ButtonElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var hoverColor: Color
    public var outlineStrokeColor: Color
    public var outlineStrokeWidth: Double
    public var borderRadius: Double
    public var content: String
    public var color: Color
    public var style: TextStyle

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, hoverColor: Color, outlineStrokeColor: Color, outlineStrokeWidth: Double, borderRadius: Double, content: String, color: Color, style: TextStyle) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.hoverColor = hoverColor
        self.outlineStrokeColor = outlineStrokeColor
        self.outlineStrokeWidth = outlineStrokeWidth
        self.borderRadius = borderRadius
        self.content = content
        self.color = color
        self.style = style
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> ButtonElement {
        ButtonElement(
            id: id,
            parentFrame: parentFrame,
            occlusionLayerId: occlusionLayerId,
            zIndex: 0,
            transform: [1, 0, 0, 1, 0, 0],
            size_x: 0,
            size_y: 0,
            hoverColor: Color(.gray),
            outlineStrokeColor: Color(.clear),
            outlineStrokeWidth: 0,
            borderRadius: 8,
            content: "",
            color: Color(.gray),
            style: defaultPaxTextStyle()
        )
    }

    public func applyPatch(_ patch: ButtonUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let hoverColor = patch.hoverColor { self.hoverColor = hoverColor }
        if let outlineStrokeColor = patch.outlineStrokeColor { self.outlineStrokeColor = outlineStrokeColor }
        if let outlineStrokeWidth = patch.outlineStrokeWidth { self.outlineStrokeWidth = outlineStrokeWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let content = patch.content { self.content = content }
        if let color = patch.color { self.color = color }
        if let style = patch.style { self.style.applyPatch(from: style) }
    }
}

public class CheckboxUpdatePatch {
    public var id: PaxNodeId
    public var background: Color?
    public var backgroundChecked: Color?
    public var outlineColor: Color?
    public var outlineWidth: Double?
    public var borderRadius: Double?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var checked: Bool?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.outlineWidth = readDouble(fb["outline_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.checked = fb["checked"]?.asBool
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
        if let backgroundChecked = fb["background_checked"], !backgroundChecked.isNull {
            self.backgroundChecked = extractColorFromBuffer(backgroundChecked)
        }
        if let outlineColor = fb["outline_color"], !outlineColor.isNull {
            self.outlineColor = extractColorFromBuffer(outlineColor)
        }
    }
}

public class CheckboxElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var background: Color
    public var backgroundChecked: Color
    public var outlineColor: Color
    public var outlineWidth: Double
    public var borderRadius: Double
    public var checked: Bool

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, background: Color, backgroundChecked: Color, outlineColor: Color, outlineWidth: Double, borderRadius: Double, checked: Bool) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.background = background
        self.backgroundChecked = backgroundChecked
        self.outlineColor = outlineColor
        self.outlineWidth = outlineWidth
        self.borderRadius = borderRadius
        self.checked = checked
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> CheckboxElement {
        CheckboxElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, background: Color(.white), backgroundChecked: Color(.blue), outlineColor: Color(.gray), outlineWidth: 1, borderRadius: 5, checked: false)
    }

    public func applyPatch(_ patch: CheckboxUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let background = patch.background { self.background = background }
        if let backgroundChecked = patch.backgroundChecked { self.backgroundChecked = backgroundChecked }
        if let outlineColor = patch.outlineColor { self.outlineColor = outlineColor }
        if let outlineWidth = patch.outlineWidth { self.outlineWidth = outlineWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let checked = patch.checked { self.checked = checked }
    }
}

public class NativeImageUpdatePatch {
    public var id: PaxNodeId
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var url: String?
    public var fit: String?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.url = fb["url"]?.asString
        self.fit = fb["fit"]?.asString
    }
}

public class NativeImageElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var url: String
    public var fit: String

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, url: String, fit: String) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.url = url
        self.fit = fit
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> NativeImageElement {
        NativeImageElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, url: "", fit: "contain")
    }

    public func applyPatch(_ patch: NativeImageUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let url = patch.url { self.url = url }
        if let fit = patch.fit { self.fit = fit }
    }
}

public class YoutubeVideoUpdatePatch {
    public var id: PaxNodeId
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var url: String?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.url = fb["url"]?.asString
    }
}

public class YoutubeVideoElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var url: String

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, url: String) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.url = url
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> YoutubeVideoElement {
        YoutubeVideoElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, url: "")
    }

    public func applyPatch(_ patch: YoutubeVideoUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let url = patch.url { self.url = url }
    }
}

public class DropdownUpdatePatch {
    public var id: PaxNodeId
    public var selectedId: UInt32?
    public var options: [String]?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var background: Color?
    public var strokeColor: Color?
    public var strokeWidth: Double?
    public var borderRadius: Double?
    public var style: TextStyleMessage?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.selectedId = readNodeId(fb["selected_id"])
        self.options = readStringArray(fb["options"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.strokeWidth = readDouble(fb["stroke_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
        if let strokeColor = fb["stroke_color"], !strokeColor.isNull {
            self.strokeColor = extractColorFromBuffer(strokeColor)
        }
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
    }
}

public class DropdownElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var selectedId: UInt32
    public var options: [String]
    public var background: Color
    public var strokeColor: Color
    public var strokeWidth: Double
    public var borderRadius: Double
    public var style: TextStyle

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, selectedId: UInt32, options: [String], background: Color, strokeColor: Color, strokeWidth: Double, borderRadius: Double, style: TextStyle) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.selectedId = selectedId
        self.options = options
        self.background = background
        self.strokeColor = strokeColor
        self.strokeWidth = strokeWidth
        self.borderRadius = borderRadius
        self.style = style
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> DropdownElement {
        DropdownElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, selectedId: 0, options: [], background: Color(.white), strokeColor: Color(.gray), strokeWidth: 1, borderRadius: 8, style: defaultPaxTextStyle())
    }

    public func applyPatch(_ patch: DropdownUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let selectedId = patch.selectedId { self.selectedId = selectedId }
        if let options = patch.options { self.options = options }
        if let background = patch.background { self.background = background }
        if let strokeColor = patch.strokeColor { self.strokeColor = strokeColor }
        if let strokeWidth = patch.strokeWidth { self.strokeWidth = strokeWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let style = patch.style { self.style.applyPatch(from: style) }
    }
}

public class RadioSetUpdatePatch {
    public var id: PaxNodeId
    public var selectedId: UInt32?
    public var options: [String]?
    public var style: TextStyleMessage?
    public var backgroundChecked: Color?
    public var outlineColor: Color?
    public var outlineWidth: Double?
    public var background: Color?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.selectedId = readNodeId(fb["selected_id"])
        self.options = readStringArray(fb["options"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.outlineWidth = readDouble(fb["outline_width"])
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
        if let backgroundChecked = fb["background_checked"], !backgroundChecked.isNull {
            self.backgroundChecked = extractColorFromBuffer(backgroundChecked)
        }
        if let outlineColor = fb["outline_color"], !outlineColor.isNull {
            self.outlineColor = extractColorFromBuffer(outlineColor)
        }
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
    }
}

public class RadioSetElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var selectedId: UInt32
    public var options: [String]
    public var style: TextStyle
    public var backgroundChecked: Color
    public var outlineColor: Color
    public var outlineWidth: Double
    public var background: Color

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, selectedId: UInt32, options: [String], style: TextStyle, backgroundChecked: Color, outlineColor: Color, outlineWidth: Double, background: Color) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.selectedId = selectedId
        self.options = options
        self.style = style
        self.backgroundChecked = backgroundChecked
        self.outlineColor = outlineColor
        self.outlineWidth = outlineWidth
        self.background = background
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> RadioSetElement {
        RadioSetElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, selectedId: 0, options: [], style: defaultPaxTextStyle(), backgroundChecked: Color(.blue), outlineColor: Color(.gray), outlineWidth: 1, background: Color(.white))
    }

    public func applyPatch(_ patch: RadioSetUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let selectedId = patch.selectedId { self.selectedId = selectedId }
        if let options = patch.options { self.options = options }
        if let backgroundChecked = patch.backgroundChecked { self.backgroundChecked = backgroundChecked }
        if let outlineColor = patch.outlineColor { self.outlineColor = outlineColor }
        if let outlineWidth = patch.outlineWidth { self.outlineWidth = outlineWidth }
        if let background = patch.background { self.background = background }
        if let style = patch.style { self.style.applyPatch(from: style) }
    }
}

public class SliderUpdatePatch {
    public var id: PaxNodeId
    public var value: Double?
    public var step: Double?
    public var min: Double?
    public var max: Double?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var accent: Color?
    public var background: Color?
    public var borderRadius: Double?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.value = readDouble(fb["value"])
        self.step = readDouble(fb["step"])
        self.min = readDouble(fb["min"])
        self.max = readDouble(fb["max"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.borderRadius = readDouble(fb["border_radius"])
        if let accent = fb["accent"], !accent.isNull {
            self.accent = extractColorFromBuffer(accent)
        }
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
    }
}

public class SliderElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var value: Double
    public var step: Double
    public var min: Double
    public var max: Double
    public var accent: Color
    public var background: Color
    public var borderRadius: Double

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, value: Double, step: Double, min: Double, max: Double, accent: Color, background: Color, borderRadius: Double) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.value = value
        self.step = step
        self.min = min
        self.max = max
        self.accent = accent
        self.background = background
        self.borderRadius = borderRadius
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> SliderElement {
        SliderElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, value: 0, step: 1, min: 0, max: 100, accent: Color(.blue), background: Color(.gray), borderRadius: 5)
    }

    public func applyPatch(_ patch: SliderUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let value = patch.value { self.value = value }
        if let step = patch.step { self.step = step }
        if let min = patch.min { self.min = min }
        if let max = patch.max { self.max = max }
        if let accent = patch.accent { self.accent = accent }
        if let background = patch.background { self.background = background }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
    }
}

public class TextboxUpdatePatch {
    public var id: PaxNodeId
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var text: String?
    public var background: Color?
    public var strokeColor: Color?
    public var strokeWidth: Double?
    public var borderRadius: Double?
    public var style: TextStyleMessage?
    public var focusOnMount: Bool?
    public var placeholder: String?
    public var outlineColor: Color?
    public var outlineWidth: Double?
    public var isTextArea: Bool?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.text = fb["text"]?.asString
        self.strokeWidth = readDouble(fb["stroke_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        self.focusOnMount = fb["focus_on_mount"]?.asBool
        self.placeholder = fb["placeholder"]?.asString
        self.outlineWidth = readDouble(fb["outline_width"])
        self.isTextArea = fb["is_text_area"]?.asBool
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
        if let strokeColor = fb["stroke_color"], !strokeColor.isNull {
            self.strokeColor = extractColorFromBuffer(strokeColor)
        }
        if let outlineColor = fb["outline_color"], !outlineColor.isNull {
            self.outlineColor = extractColorFromBuffer(outlineColor)
        }
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
    }
}

public class TextboxElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var text: String
    public var background: Color
    public var strokeColor: Color
    public var strokeWidth: Double
    public var borderRadius: Double
    public var style: TextStyle
    public var focusOnMount: Bool
    public var placeholder: String
    public var outlineColor: Color
    public var outlineWidth: Double
    public var isTextArea: Bool

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, text: String, background: Color, strokeColor: Color, strokeWidth: Double, borderRadius: Double, style: TextStyle, focusOnMount: Bool, placeholder: String, outlineColor: Color, outlineWidth: Double, isTextArea: Bool) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.text = text
        self.background = background
        self.strokeColor = strokeColor
        self.strokeWidth = strokeWidth
        self.borderRadius = borderRadius
        self.style = style
        self.focusOnMount = focusOnMount
        self.placeholder = placeholder
        self.outlineColor = outlineColor
        self.outlineWidth = outlineWidth
        self.isTextArea = isTextArea
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> TextboxElement {
        TextboxElement(id: id, parentFrame: parentFrame, occlusionLayerId: occlusionLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, text: "", background: Color(.white), strokeColor: Color(.gray), strokeWidth: 1, borderRadius: 8, style: defaultPaxTextStyle(), focusOnMount: false, placeholder: "", outlineColor: Color(.clear), outlineWidth: 0, isTextArea: false)
    }

    public func applyPatch(_ patch: TextboxUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let text = patch.text { self.text = text }
        if let background = patch.background { self.background = background }
        if let strokeColor = patch.strokeColor { self.strokeColor = strokeColor }
        if let strokeWidth = patch.strokeWidth { self.strokeWidth = strokeWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let style = patch.style { self.style.applyPatch(from: style) }
        if let focusOnMount = patch.focusOnMount { self.focusOnMount = focusOnMount }
        if let placeholder = patch.placeholder { self.placeholder = placeholder }
        if let outlineColor = patch.outlineColor { self.outlineColor = outlineColor }
        if let outlineWidth = patch.outlineWidth { self.outlineWidth = outlineWidth }
        if let isTextArea = patch.isTextArea { self.isTextArea = isTextArea }
    }
}

public class NavigationPatchMessage {
    public var url: String
    public var target: String

    public init(fb: FlxbReference) {
        self.url = fb["url"]?.asString ?? ""
        self.target = fb["target"]?.asString ?? "current"
    }
}

public class SetCursorPatchMessage {
    public var cursor: String

    public init(fb: FlxbReference) {
        self.cursor = fb["cursor"]?.asString ?? "default"
    }
}

public class LayerAddPatchMessage {
    public var numLayersToAdd: UInt32

    public init(fb: FlxbReference) {
        self.numLayersToAdd = UInt32(truncatingIfNeeded: fb["num_layers_to_add"]?.asUInt64 ?? 0)
    }
}
