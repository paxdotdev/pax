//
//  Messages.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 5/7/22.
//

import Foundation
import SwiftUI


/// Agnostic of the type of element, this patch contains only an `id_chain` field, suitable for looking up a NativeElement (e.g. for deletion)
class IdPatch {
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
    var content: String
    var transform: [Float]
    var size_x: Float
    var size_y: Float
    
    init(id_chain: [UInt64], content: String, transform: [Float], size_x: Float, size_y: Float) {
        self.id_chain = id_chain
        self.content = content
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
    }
    
    static func makeDefault(id_chain: [UInt64]) -> TextElement {
        TextElement(id_chain: id_chain, content: "", transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0)
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
    }
}



/// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
class TextUpdatePatch {
    var id_chain: [UInt64]
    var content: String?
    var transform: [Float]?
    var size_x: Float?
    var size_y: Float?
    
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
    }
}
