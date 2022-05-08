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

class TextCreatePatch {
    var id_chain: [UInt64]
    
    init(fb:FlxbReference) {
        self.id_chain = fb.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
    }

}

class TextUpdatePatch {
    var id_chain: [UInt64]
    var content: String?
    
    init(fb: FlxbReference) {
//        fb.de
        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
            fb.asUInt64!
        })
        self.content = fb["content"]?.asString
        if(self.content != nil) {
//            print(String(format: "new content for %d: %@", self.id_chain, self.content!))
        }
    }
}

class PaxCanvasView: NSView {
    
    var contextContainer : OpaquePointer? = nil
    var currentTickWorkItem : DispatchWorkItem? = nil
    
    var textElements : [[UInt64]] = [[]]
    
    func handleTextCreate(patch: TextCreatePatch) {
        textElements.append(patch.id_chain)
    }
    
    func handleTextUpdate(patch: TextUpdatePatch) {
//        print(String(format: "Handling update for %d", patch.id_chain))
    }
    
    func handleTextDelete(id: Int) {
//        textElements.removeAll(where: <#T##(Int) throws -> Bool#>)(id)
    }
    
    func processNativeMessageQueue(queue: NativeMessageQueue) {
        
//        var x : [[UInt64]: String] = [:]
//        x[[1,2,3]] = "Hello"

        let buffer = UnsafeBufferPointer<UInt8>(start: queue.data_ptr!, count: Int(queue.length))
        let root = FlexBuffer.decode(data: Data.init(buffer: buffer))!

        root["messages"]?.asVector?.makeIterator().forEach( { message in
//            print(message.debugDescription)

            let textCreateMessage = message["TextCreate"]
            if textCreateMessage != nil {
                handleTextCreate(patch: TextCreatePatch(fb: textCreateMessage!))
            }

            let textUpdateMessage = message["TextUpdate"]
            if textUpdateMessage != nil {
                handleTextUpdate(patch: TextUpdatePatch(fb: textUpdateMessage!))
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
        }
        
        DispatchQueue.main.asyncAfter(deadline: .now() + REFRESH_PERIOD, execute: currentTickWorkItem!)
    }
}
