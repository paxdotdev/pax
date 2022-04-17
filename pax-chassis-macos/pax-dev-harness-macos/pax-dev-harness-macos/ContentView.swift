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
    var tickWorkItem : DispatchWorkItem? = nil
    
    override func draw(_ dirtyRect: NSRect) {
        
        super.draw(dirtyRect)
        guard let context = NSGraphicsContext.current else { return }
        var cgContext = context.cgContext
        
        if contextContainer == nil {
            contextContainer = pax_init()
        } else {
            pax_tick(contextContainer!, &cgContext, CFloat(dirtyRect.width), CFloat(dirtyRect.height))
        }

        //This DispatchWorkItem `cancel()` is required because sometimes `draw` will be triggered externally, which
        //would otherwise create new families of DispatchWorkItems, each ticking up a frenzy, well past the bounds of our target FPS.
        //This cancellation + shared singleton (`tickWorkItem`) ensures that only one DispatchWorkItem is enqueued at a time.
        if tickWorkItem != nil {
            tickWorkItem!.cancel()
        }
        
        tickWorkItem = DispatchWorkItem {
            self.setNeedsDisplay(dirtyRect)
            self.displayIfNeeded()
        }
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD, execute: tickWorkItem!)
        
    }
}
