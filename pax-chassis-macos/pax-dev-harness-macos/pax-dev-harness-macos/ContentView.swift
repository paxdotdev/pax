//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

//struct ContentView_Previews: PreviewProvider {
//    static var previews: some View {
//        ContentView()
//    }
//}
//
//func cb (window: NSWindow?) -> Void {
//
//}
//
//struct ContentView: View {
//
//   var body: some View {
//      VStack {
//         Text("Global Sales")
//
//          MyRepresentedCustomView( callback: cb )
//      }
//   }
//}



let REFRESH_RATE = 1.0/60.0 //seconds per frame


struct ChartData {
    var array : [Int]
}

struct ContentView: View {
    
    
//    @Published var internal_date : Date
    
    var body: some View {
//        TimelineView(.periodic(from: .now, by: 1)) { context in
            CanvasViewRepresentable()
                .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
//                .onChange(of: date, perform: { _ in
//
//                })
        }
//    }
}


struct CanvasViewRepresentable: NSViewRepresentable {
//    @State var timestamp : Date
    typealias NSViewType = CanvasView
//    let date : Date
    
    func makeNSView(context: Context) -> CanvasView {
        return CanvasView()
    }
    
    func updateNSView(_ canvas: CanvasView, context: Context) {
//        canvas.tim
//        canvas.data = data
    }
}

class CanvasView: NSView {
    //TODO: figure out if this is necessary/reasonable
//    override func needsToDraw(_ rect: NSRect) -> Bool {
//        true
//    }
    
    override func draw(_ dirtyRect: NSRect) {
        print("draw call - Frame: \(self.frame)")
        
        super.draw(dirtyRect)
        
        guard let context = NSGraphicsContext.current else { return }
        
        let str = NSString(format:"hello with frame height: %f", self.frame.height)
        let rustGreetings = RustGreetings()
        print("\(rustGreetings.sayHello(to: str as String))")
        
        
        //TODO: determine order/neccessity of {saveGraphicsState, drawing/tick, and restoreGraphicsState}
        context.saveGraphicsState()

        //here!!
//        context.cgContext
        
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




//struct MyRepresentedCustomView : NSViewRepresentable {
//    typealias NSViewType = <#type#>
//
//
//}



// see: https://developer.apple.com/documentation/swiftui/nsviewrepresentable
// and https://github.com/shufflingB/swiftui-macos-windowManagment
// and https://lostmoa.com/blog/ReadingTheCurrentWindowInANewSwiftUILifecycleApp/
// and https://stackoverflow.com/questions/66982859/swiftui-nsviewrepresentable-cant-read-data-from-publisher
//
//#if canImport(AppKit)
//
//    struct MyRepresentedCustomView: NSViewRepresentable {
////        let callback: (NSWindow?) -> Void
////
////        func makeNSView(context: Self.Context) -> NSView {
////            let view = NSView()
////
////
////
////            DispatchQueue.main.async { [weak view] in
////
////                self.callback(view?.window)
////            }
////            return view
////        }
////        //coordinator likely will be needed for user input / event mapping
//////        func makeCoordinator() -> () {
//////            <#code#>
//////        }
////
////        func updateNSView(_ nsView: NSView, context: Context) {
////            let c = context.cgContext
////        }
//
//        typealias NSViewType = CanvasView
//
//            var data: ChartData //store the data -- not the chart view
//
//            func makeNSView(context: Context) -> CanvasView {
//                return CanvasView(data: data)
//            }
//
//            func updateNSView(_ chart: CanvasView, context: Context) {
//                chart.data = data //update the chart view's data
//            }
//    }
//#else
//    #error("Unsupported platform")
//#endif
//
//
