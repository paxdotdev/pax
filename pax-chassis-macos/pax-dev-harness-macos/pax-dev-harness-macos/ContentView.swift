//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

let FPS = 70.0                   //Hz
let REFRESH_PERIOD = 1.0 / FPS   //seconds between frames (e.g. 16.667 for 60Hz)

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

class FrameElements: ObservableObject {
    static let singleton : FrameElements = FrameElements()
    
    @Published var elements : [[UInt64]: FrameElement] = [:]
    
    func add(element: FrameElement) {
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
    
    
    func getPositionedTextGroup(textElement: TextElement) -> some View {
        return Group {
            Text(textElement.content)
                .foregroundColor(Color.black)
                .textSelection(.enabled)
                .frame(width: CGFloat(textElement.size_x), height: CGFloat(textElement.size_y), alignment: .topLeading)
        }
        .position(x: CGFloat(textElement.size_x) / 2.0, y: CGFloat(textElement.size_y) / 2.0)
        .transformEffect(CGAffineTransform.init(
            a: CGFloat(textElement.transform[0]),
            b: CGFloat(textElement.transform[1]),
            c: CGFloat(textElement.transform[2]),
            d: CGFloat(textElement.transform[3]),
            tx: CGFloat(textElement.transform[4]),
            ty: CGFloat(textElement.transform[5])
        ))


//
//
//            Group {
//                Text(textElement.content)
//                    .frame(width: CGFloat(textElement.size_x), height: CGFloat(textElement.size_y), alignment: .topLeading)
//                    .foregroundColor(Color.black)
////                    .background(Color.red) //debug, view bounding box
//            }
//            .transformEffect(CGAffineTransform.init(
//                a: CGFloat(textElement.transform[0]),
//                b: CGFloat(textElement.transform[1]),
//                c: CGFloat(textElement.transform[2]),
//                d: CGFloat(textElement.transform[3]),
//                tx: CGFloat(textElement.transform[4]),
//                ty: CGFloat(textElement.transform[5])
//            ))
//        }.mask(Rectangle().frame(width: CGFloat(textElement.size_x), height: CGFloat(textElement.size_y), alignment: .topLeading)).position(x: CGFloat(textElement.size_x / 2.0), y: CGFloat(textElement.size_y / 2.0))
//            .aspectRatio(contentMode: .fill)
//            .frame(width: CGFloat(textElement.size_x), height: CGFloat(textElement.size_y), alignment: .topLeading).background(Color.red)
//            .position(x: CGFloat(textElement.size_x / 2.0), y: CGFloat(textElement.size_y / 2.0))
//            .transformEffect(CGAffineTransform.init(
//                a: CGFloat(1),
//                b: CGFloat(0),
//                c: CGFloat(0),
//                d: CGFloat(1),
//                tx: CGFloat(textElement.transform[4]),
//                ty: CGFloat(textElement.transform[5])
//            )).clipped()// //correct for "center anchor" in SwiftUI
    }
  
    var body: some View {
        ZStack{
            
            //Clipping algo:
            //for each clipping container
            // instantiate container, then instantiate all of its direct descendants
            //   if its descendants include another clipping container, recurse
            //finally, render the set of elements without a clipping container
            ForEach(Array(self.textElements.elements.values), id: \.id_chain) { textElement in
                getPositionedTextGroup(textElement: textElement)
            }
        }
        
        
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
    @ObservedObject var frameElements = FrameElements.singleton
    
    var contextContainer : OpaquePointer? = nil
    var currentTickWorkItem : DispatchWorkItem? = nil    
    
    func handleTextCreate(patch: AnyCreatePatch) {
        textElements.add(element: TextElement.makeDefault(id_chain: patch.id_chain))
    }
    
    func handleTextUpdate(patch: TextUpdatePatch) {
        textElements.elements[patch.id_chain]?.applyPatch(patch: patch)
        textElements.objectWillChange.send()
    }
    
    func handleTextDelete(patch: AnyDeletePatch) {
        textElements.remove(id: patch.id_chain)
    }
    
    func handleFrameCreate(patch: AnyCreatePatch) {
        frameElements.add(element: FrameElement.makeDefault(id_chain: patch.id_chain))
    }
    
    func handleFrameUpdate(patch: FrameUpdatePatch) {
        frameElements.elements[patch.id_chain]?.applyPatch(patch: patch)
        frameElements.objectWillChange.send()
    }
    
    func handleFrameDelete(patch: AnyDeletePatch) {
        frameElements.remove(id: patch.id_chain)
    }
    
    func processNativeMessageQueue(queue: NativeMessageQueue) {

        let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
        let root = FlexBuffer.decode(data: Data.init(buffer: buffer))!

        root["messages"]?.asVector?.makeIterator().forEach( { message in

            let textCreateMessage = message["TextCreate"]
            if textCreateMessage != nil {
                handleTextCreate(patch: AnyCreatePatch(fb: textCreateMessage!))
            }

            let textUpdateMessage = message["TextUpdate"]
            if textUpdateMessage != nil {
                handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage!))
            }
            
            let textDeleteMessage = message["TextDelete"]
            if textDeleteMessage != nil {
                handleTextDelete(patch: AnyDeletePatch(fb: textDeleteMessage!))
            }
            
            let frameCreateMessage = message["FrameCreate"]
            if frameCreateMessage != nil {
                //NOTE: using AnyDeletePatch as a hack -- frames can only be associated with other frames so they're treated
                handleFrameCreate(patch: AnyCreatePatch(fb: frameCreateMessage!))
            }

            let frameUpdateMessage = message["FrameUpdate"]
            if frameUpdateMessage != nil {
                handleFrameUpdate(patch: FrameUpdatePatch(fb: frameUpdateMessage!))
            }
            
            let frameDeleteMessage = message["FrameDelete"]
            if frameDeleteMessage != nil {
                handleFrameDelete(patch: AnyDeletePatch(fb: frameDeleteMessage!))
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
        
        //This DispatchWorkItem `cancel()` is required because sometimes `draw` will be triggered externally from this loop, which
        //would otherwise create new families of continuously reproducing DispatchWorkItems, each ticking up a frenzy, well past the bounds of our target FPS.
        //This cancellation + shared singleton (`tickWorkItem`) ensures that only one DispatchWorkItem is enqueued at a time.
        if currentTickWorkItem != nil {
            currentTickWorkItem!.cancel()
        }
        
        currentTickWorkItem = DispatchWorkItem {
            self.setNeedsDisplay(dirtyRect)
            self.displayIfNeeded()
        }
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD, execute: currentTickWorkItem!)
    }
}
