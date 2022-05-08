//
//  Messages.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 5/7/22.
//

import Foundation

class TextIdPatch {
    var id_chain: [UInt64]
    
    init(fb:FlxbReference) {
        self.id_chain = fb.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
    }

}

struct TextElement {
    var id_chain: [UInt64]
    var content: String
    var transform: [Float]
    var size_x: Float
    var size_y: Float
    
    static func make_default(id_chain: [UInt64]) -> Self {
        Self(id_chain: id_chain, content: "", transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0)
    }
}

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
