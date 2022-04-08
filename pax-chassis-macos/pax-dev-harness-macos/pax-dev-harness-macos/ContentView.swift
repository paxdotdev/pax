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





struct ChartData {
    var array : [Int]
}

struct ContentView: View {
    @State var chartData : ChartData = ChartData(array: [])
    
    var body: some View {
        Group {
            ChartViewRepresentable(data: chartData)
                .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
        }
        .onAppear{
            DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
                print("New data!")
                chartData = ChartData(array: [1,2,3,4])
            }
        }
    }
}


struct ChartViewRepresentable: NSViewRepresentable {
    typealias NSViewType = ChartView
    
    var data: ChartData
    
    func makeNSView(context: Context) -> ChartView {
        return ChartView(data: data)
    }
    
    func updateNSView(_ chart: ChartView, context: Context) {
        chart.data = data
    }
}

class ChartView: NSView {
    
    var data: ChartData {
        didSet {
            self.needsDisplay = true
        }
    }
    
    init(data: ChartData) {
        self.data = data
        print("\(data)")
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = .white
    }
    
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
    
    override func draw(_ dirtyRect: NSRect) {
        print("draw call - Frame: \(self.frame), Data: \(data.array.count)")
        
        super.draw(dirtyRect)
        
        guard let context = NSGraphicsContext.current else { return }
        context.saveGraphicsState()

        //here!!
//        context.cgContext
        
        //TODO: pass cgContext pointer to pax-chassis-macos if this is the first `draw`
        //      (ideally, there would be a separate lifecycle method
        
        if data.array.count > 0 {
            //detect data present on ChartView
            let ctx = context.cgContext
            ctx.setFillColor(NSColor.green.cgColor)
            ctx.fillEllipse(in: CGRect(x: 10, y: 10, width: 10, height: 10))
        }
        
        context.restoreGraphicsState()
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

#if canImport(AppKit)
    
    struct MyRepresentedCustomView: NSViewRepresentable {
//        let callback: (NSWindow?) -> Void
//
//        func makeNSView(context: Self.Context) -> NSView {
//            let view = NSView()
//
//
//
//            DispatchQueue.main.async { [weak view] in
//
//                self.callback(view?.window)
//            }
//            return view
//        }
//        //coordinator likely will be needed for user input / event mapping
////        func makeCoordinator() -> () {
////            <#code#>
////        }
//
//        func updateNSView(_ nsView: NSView, context: Context) {
//            let c = context.cgContext
//        }
        
        typealias NSViewType = ChartView
            
            var data: ChartData //store the data -- not the chart view
            
            func makeNSView(context: Context) -> ChartView {
                return ChartView(data: data)
            }
            
            func updateNSView(_ chart: ChartView, context: Context) {
                chart.data = data //update the chart view's data
            }
    }
#else
    #error("Unsupported platform")
#endif


