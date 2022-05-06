//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

let FPS = 60.0                   //Hz
let REFRESH_PERIOD = 1.0 / FPS   //seconds between frames (e.g. 16.667 for 60Hz)

struct ContentView: View {
    var body: some View {
        PaxCanvasViewRepresentable()
            .frame(minWidth: 300, maxWidth: .infinity, minHeight: 300, maxHeight: .infinity)
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
    
    var contextContainer : OpaquePointer? = nil
    var currentTickWorkItem : DispatchWorkItem? = nil
    
    var textElements : [Int] = []
    
    func handleTextCreate(id: Int) {
        textElements.append(id)
    }
    
    
//    typedef struct TextPatch {
//      struct COption_CString content;
//      struct COption_Affine transform;
//      struct COption_TextSize size_x;
//      struct COption_TextSize size_y;
//    } TextPatch;
    
    func handleTextUpdate(params: TextUpdate_Body) {
        let instance_id = params._0
        let patch = params._1
        
        let text_node = () //TODO: Look up from pool
        
//        switch patch.content.tag {
//            case Some_CString:
//                let new_content = String(cString: patch.content.some.pointee!)
//                print("new content: " + new_content)
//            default:
//                ()
//        }
    }
    
    func handleTextDelete(id: Int) {
//        textElements.removeAll(where: <#T##(Int) throws -> Bool#>)(id)
    }
    
    func processNativeMessageQueue(queue: NativeMessageQueue) {
        let arr = UnsafeBufferPointer<NativeMessage>(start: queue.msg_ptr, count: Int(queue.length))
        arr.forEach { msg in
            switch msg.tag {
                case TextCreate:
                    let instance_id = msg.text_create
                    handleTextCreate(id: Int(instance_id))
                case TextUpdate:
                    let update_params = msg.text_update
                    handleTextUpdate(params: update_params)
                case TextDelete:
                    let instance_id = msg.text_delete
                    handleTextDelete(id: Int(instance_id))
                case ClippingCreate:
                    ()
                case ClippingUpdate:
                    ()
                case ClippingDelete:
                    ()
                default:
                    print("unrecognized message type")
            }
        }
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
//            pax_cleanup_message_queue(nativeMessageQueue)
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
        }
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD, execute: currentTickWorkItem!)
    }
}
