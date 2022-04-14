//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI



let REFRESH_RATE = 1.0/60.0 //seconds per frame


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
    
    override func draw(_ dirtyRect: NSRect) {
        print("draw call - Frame: \(self.frame)")
        
        super.draw(dirtyRect)
        
        guard let context = NSGraphicsContext.current else { return }
        
        let str = NSString(format:"with frame height: %f", self.frame.height)
        let rustGreetings = RustGreetings()
        print("\(rustGreetings.sayHello(to: str as String))")
        
        
        //TODO: determine order/neccessity of {saveGraphicsState, drawing/tick, and restoreGraphicsState}
        context.saveGraphicsState()

        //here!!
        let cgContext = context.cgContext
        
        //TODO: pass cgContext pointer to pax-chassis-macos if this is the first `draw`
        //      (TODO: ideally, this would be best sent in a separate lifecycle method, e.g. `init`. Someone who knows SwiftUI should refactor & improve: pass CGContext pointer on `init`, then call `tick` on `draw`)
        //       Alternatively, if the cgContext pointer moves between ticks, support a new context per tick in chassis-macos (and probably chassis-ios)
        //TODO: send `tick` event to pax-chassis-macos
        
        context.restoreGraphicsState()
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_RATE) {
            self.setNeedsDisplay(dirtyRect)
            self.displayIfNeeded()
        }
    }
}

class RustGreetings {
    func sayHello(to: String) -> String {
        let result = rust_greeting(to)
        let swift_result = String(cString: result!)
        rust_greeting_free(UnsafeMutablePointer(mutating: result))
        return swift_result
    }
}



// see: https://developer.apple.com/documentation/swiftui/nsviewrepresentable
// and https://github.com/shufflingB/swiftui-macos-windowManagment
// and https://lostmoa.com/blog/ReadingTheCurrentWindowInANewSwiftUILifecycleApp/
// and https://stackoverflow.com/questions/66982859/swiftui-nsviewrepresentable-cant-read-data-from-publisher
