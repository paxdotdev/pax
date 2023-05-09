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
    var font_spec: FontSpec
    var fill: Color
    var paragraphAlignment: TextAlignment
    var alignment: Alignment
    var autoResize: Bool
    
    init(id_chain: [UInt64], clipping_ids: [[UInt64]], content: String, transform: [Float], size_x: Float, size_y: Float, font: FontSpec, fill: Color, paragraphAlignment: TextAlignment, alignment: Alignment, autoResize: Bool) {
        self.id_chain = id_chain
        self.clipping_ids = clipping_ids
        self.content = content
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.font_spec = font
        self.fill = fill
        self.paragraphAlignment = paragraphAlignment
        self.alignment = alignment
        self.autoResize = autoResize
    }
    
    static func makeDefault(id_chain: [UInt64], clipping_ids: [[UInt64]]) -> TextElement {
        TextElement(id_chain: id_chain, clipping_ids: clipping_ids, content: "", transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0, font: FontSpec(), fill: Color(.black), paragraphAlignment: .leading, alignment: .topLeading, autoResize: false)
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
        
        if patch.paragraphAlignment != nil {
            self.paragraphAlignment = patch.paragraphAlignment!.toTextAlignment()
        }
        if patch.verticalAlignment != nil && patch.horizontalAlignment != nil {
            self.alignment = toAlignment(horizontalAlignment: patch.horizontalAlignment!, verticalAlignment: patch.verticalAlignment!)
        }
        if patch.autoResize != nil {
            self.autoResize = patch.autoResize!
        }
    }
}

enum HAlignment {
    case center
    case left
    case right
}

extension HAlignment {
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

enum VAlignment {
    case top
    case center
    case bottom
}


func toAlignment(horizontalAlignment: HAlignment, verticalAlignment: VAlignment) -> Alignment {
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



/// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
class TextUpdatePatch {
    var id_chain: [UInt64]
    var content: String?
    var transform: [Float]?
    var size_x: Float?
    var size_y: Float?

    var fontBuffer: FlxbReference
    var fill: Color?
    
    var paragraphAlignment: HAlignment?
    var verticalAlignment: VAlignment?
    var horizontalAlignment: HAlignment?
    var autoResize: Bool?

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
        
        if fb["fill"] != nil && !fb["fill"]!.isNull {
            if fb["fill"]!["Rgba"] != nil && !fb["fill"]!["Rgba"]!.isNull {
                let stub = fb["fill"]!["Rgba"]!
                self.fill = Color(
                    red: Double(stub[0]!.asFloat!),
                    green: Double(stub[1]!.asFloat!),
                    blue: Double(stub[2]!.asFloat!),
                    opacity: Double(stub[3]!.asFloat!)
                )
            } else {
                let stub = fb["fill"]!["Hlca"]!
                self.fill = Color(
                    hue: Double(stub[0]!.asFloat!),
                    saturation: Double(stub[1]!.asFloat!),
                    brightness: Double(stub[2]!.asFloat!),
                    opacity: Double(stub[3]!.asFloat!)
                )
            }
        }
        
        if let alignmentValue = fb["paragraph_alignment"]?.asString {
            switch alignmentValue {
            case "Center":
                self.paragraphAlignment = .center
            case "Left":
                self.paragraphAlignment = .left
            case "Right":
                self.paragraphAlignment = .right
            default:
                self.paragraphAlignment = nil
            }
        }
        
        if let verticalAlignmentValue = fb["vertical_alignment"]?.asString {
            switch verticalAlignmentValue {
            case "Top":
                self.verticalAlignment = .top
            case "Center":
                self.verticalAlignment = .center
            case "Bottom":
                self.verticalAlignment = .bottom
            default:
                self.verticalAlignment = nil
            }
        }
        
        if let alignmentValue = fb["horizontal_alignment"]?.asString {
            switch alignmentValue {
            case "Center":
                self.horizontalAlignment = .center
            case "Left":
                self.horizontalAlignment = .left
            case "Right":
                self.horizontalAlignment = .right
            default:
                self.horizontalAlignment = nil
            }
        }
        
        if let boundingBoxValue = fb["bounding_box"]?.asString {
            switch boundingBoxValue {
            case "Fixed":
                self.autoResize = false
            case "Auto":
                self.autoResize = true
            default:
                self.autoResize = false
            }
        }
    }
}


class FontSpec {
    var family: String
    var variant: String
    var size: Float
    var cachedFont: Font?
    
    func applyPatch(fb: FlxbReference) {
        if fb["variant"] != nil && !fb["variant"]!.isNull {
            self.variant = fb["variant"]!.asString!
        }
        if fb["family"] != nil && !fb["family"]!.isNull {
            self.family = fb["family"]!.asString!
        }
        if fb["size"] != nil && !fb["size"]!.isNull {
            self.size = fb["size"]!.asFloat!
        }
        
        self.cachedFont = self.intoFont()
    }
    
    init() {
        self.family = "Arial"
        self.variant = "Regular"
        self.size = 64.0
        self.cachedFont = self.intoFont()
    }
    
    private func intoFont() -> Font {
        var suffix = ""
        if self.variant != "Regular" {
            suffix = " " + self.variant
        }
        return Font.custom(String(self.family + suffix), size: CGFloat(self.size)).width(Font.Width.expanded)
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

