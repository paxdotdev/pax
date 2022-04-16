//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI



let REFRESH_PERIOD = 1.0/60 //seconds per frame


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
    
    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        
        guard let context = NSGraphicsContext.current else { return }
        
        //TODO: determine order/neccessity of {saveGraphicsState, drawing/tick, and restoreGraphicsState}
        context.saveGraphicsState()

        var cgContext = context.cgContext
        
        if let initializedContainer = contextContainer {
//            print("running tick with context at address: \(cgContext)")
            pax_tick(initializedContainer, &cgContext)
        } else {
//            print("initializing contextContainer")
            contextContainer = pax_init()
        }
        
        context.restoreGraphicsState()
        
        //TODO: use TimelineView or better to handle render loop.
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD) {
            self.setNeedsDisplay(dirtyRect)
            self.displayIfNeeded()
        }
    }
}
//
//class RustGreetings {
//    func sayHello(to: String) -> String {
//        let result = rust_greeting(to)
//        let swift_result = String(cString: result!)
//        rust_greeting_free(UnsafeMutablePointer(mutating: result))
//        return swift_result
//    }
//}



// see: https://developer.apple.com/documentation/swiftui/nsviewrepresentable
// and https://github.com/shufflingB/swiftui-macos-windowManagment
// and https://lostmoa.com/blog/ReadingTheCurrentWindowInANewSwiftUILifecycleApp/
// and https://stackoverflow.com/questions/66982859/swiftui-nsviewrepresentable-cant-read-data-from-publisher
