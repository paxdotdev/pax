//
//  ContentView.swift
//  Shared
//
//  Created by Zachary Brown on 4/3/22.
//






import SwiftUI

struct ContentView: View {

    var body: some CustomView {

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
    
//    override func drawRect(dirtyRect: NSRect) {
//        NSColor.redColor().set() // choose color
//        let figure = NSBezierPath() // container for line(s)
//        figure.moveToPoint(NSMakePoint(x, y)) // start point
//        figure.lineToPoint(NSMakePoint(x+10.0, y+10.0)) // destination
//        figure.lineWidth = 1  // hair line
//        figure.stroke()  // draw line(s) in color
//      }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        
        NSColor.red.set() // choose color
        let figure = NSBezierPath() // container for line(s)
        figure.appendArc(from: NSMakePoint(50, 50), to: NSMakePoint(250, 250), radius: 50.0)
        figure.lineWidth = 1  // hair line
        figure.stroke()  // draw line(s) in color

//        saveGState { ctx in
//            //todo: tick engine
//            // Drawing code here.
//        }
        
    }
}

