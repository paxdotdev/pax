//
//  ContentView.swift
//  Shared
//
//  Created by Zachary Brown on 4/3/22.
//



import SwiftUI

struct ContentView: View {
    
    var body: some View {
        
        
        
        Text("Hello, world!")
            .padding()
    }
}


class CustomView: NSView {

    private var currentContext : CGContext? {
        get {
            if #available(OSX 10.10, *) {
                return NSGraphicsContext.current?.cgContext
            } else if let contextPointer = NSGraphicsContext.current?.graphicsPort {
                return Unmanaged.fromOpaque(contextPointer).takeUnretainedValue()
            }

            return nil
        }
    }

//    private func saveGState(drawStuff: (_ ctx: CGContext) -> ()) -> () {
//        if let context = self.currentContext {
//            CGContext.saveGState(context)
//            drawStuff(context)
//            context.restoreGState ()
//        }
//    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)

        saveGState { ctx in
            //todo: tick engine
            // Drawing code here.
        }
    }
}

