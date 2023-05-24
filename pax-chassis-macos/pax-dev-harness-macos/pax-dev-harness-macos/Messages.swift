//
//  Messages.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 5/7/22.
//

import Foundation
import SwiftUI


/// Agnostic of the type of element, this patch contains only an `id_chain` field, suitable for looking up a NativeElement (e.g. for deletion)
class AnyCreatePatch {
    var id_chain: [UInt64]
    /// Used for clipping -- each `[UInt64]` is an `id_chain` for an associated clipping mask (`Frame`)
    var clipping_ids: [[UInt64]]
    
    init(fb:FlxbReference) {
        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
        
        self.clipping_ids = fb["clipping_ids"]!.asVector!.makeIterator().map({ fb in
            fb.asVector!.makeIterator().map({ fb in
                fb.asUInt64!
            })
        })
    }
}


class AnyDeletePatch {
    var id_chain: [UInt64]
    
    init(fb:FlxbReference) {
        self.id_chain = fb.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
        
    }
}


/// Represents a native Text element, as received by message patches from Pax core
class TextElement {
    var id_chain: [UInt64]
    var clipping_ids: [[UInt64]]
    var content: String
    var transform: [Float]
    var size_x: Float
    var size_y: Float
    var font_spec: PaxFont
    var fill: Color
    var alignmentMultiline: TextAlignment
    var alignment: Alignment
    // New properties
    var size: CGFloat
    var style_link: LinkStyle?
    
    init(id_chain: [UInt64], clipping_ids: [[UInt64]], content: String, transform: [Float], size_x: Float, size_y: Float, font: PaxFont, fill: Color, alignmentMultiline: TextAlignment, alignment: Alignment, size: CGFloat) {
        self.id_chain = id_chain
        self.clipping_ids = clipping_ids
        self.content = content
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.font_spec = font
        self.fill = fill
        self.alignmentMultiline = alignmentMultiline
        self.alignment = alignment
        self.size = size
    }
    
    static func makeDefault(id_chain: [UInt64], clipping_ids: [[UInt64]]) -> TextElement {
        TextElement(id_chain: id_chain, clipping_ids: clipping_ids, content: "", transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0, font: PaxFont.makeDefault(), fill: Color(.black), alignmentMultiline: .leading, alignment: .topLeading, size: 0.0)
    }
    
    func applyPatch(patch: TextUpdatePatch) {
        //no-op to ID, as it is primary key
        
        if patch.content != nil {
            self.content = patch.content!
        }
        if patch.transform != nil {
            self.transform = patch.transform!
        }
        if patch.size_x != nil {
            self.size_x = patch.size_x!
        }
        if patch.size_y != nil {
            self.size_y = patch.size_y!
        }
        
        self.font_spec.applyPatch(fb: patch.fontBuffer)

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
        
        // Apply new properties
        if patch.size != nil {
            self.size = patch.size!
        }
        self.style_link = patch.style_link
    }
}

enum TextAlignHorizontal {
    case center
    case left
    case right
}

extension TextAlignHorizontal {
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

enum TextAlignVertical {
    case top
    case center
    case bottom
}


func toAlignment(horizontalAlignment: TextAlignHorizontal, verticalAlignment: TextAlignVertical) -> Alignment {
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
class ImageLoadPatch {
    var id_chain: [UInt64]
    var path: String?
    
    init(fb:FlxbReference) {
        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
        self.path = fb["path"]?.asString
    }
}

/// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
class TextUpdatePatch {
    var id_chain: [UInt64]
    var content: String?
    var transform: [Float]?
    var size_x: Float?
    var size_y: Float?
    var fontBuffer: FlxbReference
    var fill: Color?
    var align_multiline: TextAlignHorizontal?
    var align_vertical: TextAlignVertical?
    var align_horizontal: TextAlignHorizontal?
    // New properties
    var size: CGFloat?
    var style_link: LinkStyle?

    init(fb: FlxbReference) {
        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
        self.content = fb["content"]?.asString
        self.transform = fb["transform"]?.asVector?.makeIterator().map({ fb in
            fb.asFloat!
        })
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.fontBuffer =  fb["font"]!
        
        if let fillBuffer = fb["fill"], !fillBuffer.isNull {
            self.fill = extractColorFromBuffer(fillBuffer)
        }
        
        if let alignmentValue = fb["align_multiline"]?.asString {
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
        
        if let verticalAlignmentValue = fb["align_vertical"]?.asString {
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
        
        if let alignmentValue = fb["align_horizontal"]?.asString {
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
        
        self.size = fb["size"]?.asFloat.map { CGFloat($0) }
        
        if !fb["style_link"]!.isNull {
            self.style_link = LinkStyle(fb: fb["style_link"]!)
        }
   
    }
}


func extractColorFromBuffer(_ fillBuffer: FlxbReference) -> Color {
    if let rgba = fillBuffer["Rgba"], !rgba.isNull {
        let stub = fillBuffer["Rgba"]!
        return Color(
            red: Double(stub[0]!.asFloat!),
            green: Double(stub[1]!.asFloat!),
            blue: Double(stub[2]!.asFloat!),
            opacity: Double(stub[3]!.asFloat!)
        )
    } else if let hlc = fillBuffer["Hlca"], !hlc.isNull {
        let stub = fillBuffer["Hlca"]!
        return Color(
            hue: Double(stub[0]!.asFloat!),
            saturation: Double(stub[1]!.asFloat!),
            brightness: Double(stub[2]!.asFloat!),
            opacity: Double(stub[3]!.asFloat!)
        )
    } else {
        return Color.black
    }
}

class LinkStyle {
    var font: PaxFont
    var fill: Color
    var underline: Bool
    var size: CGFloat

    init(fb: FlxbReference) {
        self.font = PaxFont.makeDefault()
        self.font.applyPatch(fb: fb["font"]!)
        self.fill = extractColorFromBuffer(fb["fill"]!)
        self.underline = fb["underline"]?.asBool ?? false
        self.size = CGFloat(fb["size"]?.asFloat ?? 12)
    }
}

enum TextAlignHorizontalMessage: String {
    case Left, Center, Right
}

enum FontStyle: String {
    case normal = "Normal"
    case italic = "Italic"
    case oblique = "Oblique"
}

extension FontWeight {
    func fontWeight() -> Font.Weight {
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

enum FontWeight: String {
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

class PaxFont {
    enum PaxFontType {
        case system(SystemFont)
        case web(WebFont)
        case local(LocalFont)
    }

    struct SystemFont {
        let family: String
        let style: FontStyle
        let weight: FontWeight
    }

    struct WebFont {
        let family: String
        let url: URL
        let style: FontStyle
        let weight: FontWeight
    }

    struct LocalFont {
        let family: String
        let path: URL
        let style: FontStyle
        let weight: FontWeight
    }

    var type: PaxFontType
    var cachedFont: Font?
    var currentSize: CGFloat

    init(type: PaxFontType) {
        self.type = type
        self.currentSize = 12
    }
    
    static func makeDefault() -> PaxFont {
        let defaultSystemFont = SystemFont(family: "Helvetica", style: .normal, weight: .normal)
        return PaxFont(type: .system(defaultSystemFont))
    }
    
    func getFont(size: CGFloat) -> Font {
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



    func applyPatch(fb: FlxbReference) {
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

    static func isFontRegistered(fontFamily: String) -> Bool {
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
}
//
//class FontFactory {
////    var family: String
////    var variant: String
////    var size: Float
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



class FrameElement {
    var id_chain: [UInt64]
    var transform: [Float]
    var size_x: Float
    var size_y: Float
    
    init(id_chain: [UInt64], transform: [Float], size_x: Float, size_y: Float) {
        self.id_chain = id_chain
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
    }
    
    static func makeDefault(id_chain: [UInt64]) -> FrameElement {
        FrameElement(id_chain: id_chain, transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0)
    }
    
    func applyPatch(patch: FrameUpdatePatch) {
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
    }
}



/// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
class FrameUpdatePatch {
    var id_chain: [UInt64]
    var transform: [Float]?
    var size_x: Float?
    var size_y: Float?
    
    init(fb: FlxbReference) {
        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
        self.transform = fb["transform"]?.asVector?.makeIterator().map({ fb in
            fb.asFloat!
        })
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
    }
}

