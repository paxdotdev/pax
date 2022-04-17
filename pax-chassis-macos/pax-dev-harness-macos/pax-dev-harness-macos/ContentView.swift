//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

let FPS = 60.0
let REFRESH_PERIOD = 1.0/FPS //seconds between frames (e.g. 16.667 for 60Hz)

struct ContentView: View {
    var body: some View {
        CanvasViewRepresentable()
            .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
    }
}


struct CanvasViewRepresentable: NSViewRepresentable {
    typealias NSViewType = CanvasView
    
    func makeNSView(context: Context) -> CanvasView {
        return CanvasView()
    }
    
    func updateNSView(_ canvas: CanvasView, context: Context) {
    }
}

class CanvasView: NSView {
    
    var contextContainer : OpaquePointer? = nil
    var needsDispatch : Bool = true
    
    override func draw(_ dirtyRect: NSRect) {
        
        super.draw(dirtyRect)
        guard let context = NSGraphicsContext.current else { return }
        var cgContext = context.cgContext
        
        if let initializedContainer = contextContainer {
            pax_tick(initializedContainer, &cgContext, CFloat(dirtyRect.width), CFloat(dirtyRect.height))
        } else {
            contextContainer = pax_init()
        }
        
        //needsDispatch is used as a hack to keep DispatchQueue workitems from multiplying, e.g.
        //when `draw` is triggered by a window resize (where each event will create its own family of workitems)
        //
        //This might leave edge-cases where resizing leaves bounds calculations off-by-a-frame, and a more robust approach
        //might be to use the WorkItem API to cancel/make sure that the latest need-to-refresh wins
        if needsDispatch {
            needsDispatch = false
            DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD) {
                self.needsDispatch = true
                
                //TODO: use TimelineView or CVDisplayLink (or something better?) to handle this "clock signal" for render loop.
                self.setNeedsDisplay(dirtyRect)
                self.displayIfNeeded()
            }
        }
    }
}
