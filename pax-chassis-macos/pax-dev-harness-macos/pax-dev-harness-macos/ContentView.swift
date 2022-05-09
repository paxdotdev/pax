//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

let FPS = 60.0                   //Hz
let REFRESH_PERIOD = 1.0 / FPS   //seconds between frames (e.g. 16.667 for 60Hz)
var tick = 0

//var textElements


class TextElements: ObservableObject {
    static let singleton : TextElements = TextElements()
    
    @Published var elements : [[UInt64]: TextElement] = [:]
    
    func add(element: TextElement) {
        self.elements[element.id_chain] = element
    }
    func remove(id: [UInt64]) {
        self.elements.removeValue(forKey: id)
    }
}

struct ContentView: View {
    var body: some View {
        ZStack {
            PaxCanvasViewRepresentable()
                .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
            NativeRenderingLayer()
        }
    }
}


struct NativeRenderingLayer: View {
    
    @ObservedObject var textElements : TextElements = TextElements.singleton
  
    var body: some View {
        ZStack{
            ForEach(Array(self.textElements.elements.values), id: \.id_chain) { textElement in
                Text(textElement.content)
                    .frame(width: CGFloat(textElement.size_x), height: CGFloat(textElement.size_y), alignment: .topLeading)
//                    .position(CGPoint(x:200.0, y:200.0))
                    .background(Color.red)
    //                .transformEffect(CGAffineTransform.init(
    //                    a: CGFloat(textElement.transform[0]),
    //                    b: CGFloat(textElement.transform[1]),
    //                    c: CGFloat(textElement.transform[2]),
    //                    d: CGFloat(textElement.transform[3]),
    //                    tx: CGFloat(textElement.transform[4]),
    //                    ty: CGFloat(textElement.transform[5])
    //                ))
            }
        }
        
//        ForEach(0..<keys.count) { index in
//            let key = keys[index]
//            let textElement = textElements[key]!
//            Text(textElement.content)
//                .frame(width: 100.0, height: 100.0, alignment: .topLeading)
//                .background(Color.red)
//                .transformEffect(CGAffineTransform.init(
//                    a: CGFloat(textElement.transform[0]),
//                    b: CGFloat(textElement.transform[1]),
//                    c: CGFloat(textElement.transform[2]),
//                    d: CGFloat(textElement.transform[3]),
//                    tx: CGFloat(textElement.transform[4]),
//                    ty: CGFloat(textElement.transform[5])
//                ))
//        }
        
    }
}

struct PaxCanvasViewRepresentable: NSViewRepresentable {
    typealias NSViewType = PaxCanvasView
    
    func makeNSView(context: Context) -> PaxCanvasView {
        let view = PaxCanvasView()
        //TODO: BG transparency
        return view
    }
    
    func updateNSView(_ canvas: PaxCanvasView, context: Context) { }
}


class PaxCanvasView: NSView {
    
    @ObservedObject var textElements = TextElements.singleton
    
    var contextContainer : OpaquePointer? = nil
    var currentTickWorkItem : DispatchWorkItem? = nil    
    
    func handleTextCreate(patch: TextIdPatch) {
        textElements.add(element: TextElement.makeDefault(id_chain: patch.id_chain))
    }
    
    func handleTextUpdate(patch: TextUpdatePatch) {
        textElements.elements[patch.id_chain]?.applyPatch(patch: patch)
        textElements.objectWillChange.send()
    }
    
    func handleTextDelete(patch: TextIdPatch) {
        textElements.remove(id: patch.id_chain)
    }
    
    func processNativeMessageQueue(queue: NativeMessageQueue) {

        let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
        let root = FlexBuffer.decode(data: Data.init(buffer: buffer))!

        root["messages"]?.asVector?.makeIterator().forEach( { message in

            let textCreateMessage = message["TextCreate"]
            if textCreateMessage != nil {
                handleTextCreate(patch: TextIdPatch(fb: textCreateMessage!))
            }

            let textUpdateMessage = message["TextUpdate"]
            if textUpdateMessage != nil {
                handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage!))
            }
            
            let textDeleteMessage = message["TextDelete"]
            if textDeleteMessage != nil {
                handleTextDelete(patch: TextIdPatch(fb: textDeleteMessage!))
            }

            //^ Add new message-receive handlers here ^
        })
        
    }
    
    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard let context = NSGraphicsContext.current else { return }
        var cgContext = context.cgContext
        
        if contextContainer == nil {
            let swiftLoggerCallback : @convention(c) (UnsafePointer<CChar>?) -> () = {
                (msg) -> () in
                let outputString = String(cString: msg!)
                print(outputString)
            }
            
//            print("Sleeping 10 seconds to allow manual debugger attachment...")
//            sleep(10)

            contextContainer = pax_init(swiftLoggerCallback)
        } else {
            
            let nativeMessageQueue = pax_tick(contextContainer!, &cgContext, CFloat(dirtyRect.width), CFloat(dirtyRect.height))
            processNativeMessageQueue(queue: nativeMessageQueue.unsafelyUnwrapped.pointee)
            pax_dealloc_message_queue(nativeMessageQueue)
        }
        
        
        //Render populated native elements
//        print(textElements)
        
        

        //This DispatchWorkItem `cancel()` is required because sometimes `draw` will be triggered externally from this loop, which
        //would otherwise create new families of continuously reproducing DispatchWorkItems, each ticking up a frenzy, well past the bounds of our target FPS.
        //This cancellation + shared singleton (`tickWorkItem`) ensures that only one DispatchWorkItem is enqueued at a time.
        if currentTickWorkItem != nil {
            currentTickWorkItem!.cancel()
        }
        
        currentTickWorkItem = DispatchWorkItem {
            self.setNeedsDisplay(dirtyRect)
            self.displayIfNeeded()
            tick = tick + 1
        }
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD, execute: currentTickWorkItem!)
    }
}
