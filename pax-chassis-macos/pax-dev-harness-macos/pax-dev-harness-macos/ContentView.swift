//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

let FPS = 60.0
let REFRESH_PERIOD = 1.0/FPS //seconds between frames (e.g. 16.667 for 60Hz)

struct ChartData {
    var array : [Int]
}

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
            pax_tick(initializedContainer, &cgContext)
        } else {
            contextContainer = pax_init()
        }
        
        //TODO: use TimelineView or CVDisplayLink (or something better?) to handle "clock signal" for render loop.
        //TODO: fix multiplying dispatchqueue events when `draw` is called externally (e.g. with window resizing)
        //      see https://stackoverflow.com/questions/48016111/how-to-stop-a-dispatchqueue-in-swift for a potential solution
        //      -- also consider an approach outside of DispatchQueue
        
        if needsDispatch {
            needsDispatch = false
            DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD) {
                self.needsDispatch = true
                self.setNeedsDisplay(dirtyRect)
                self.displayIfNeeded()
            }
        }
    }
}


// see: https://developer.apple.com/documentation/swiftui/nsviewrepresentable
// and https://github.com/shufflingB/swiftui-macos-windowManagment
// and https://lostmoa.com/blog/ReadingTheCurrentWindowInANewSwiftUILifecycleApp/
// and https://stackoverflow.com/questions/66982859/swiftui-nsviewrepresentable-cant-read-data-from-publisher
