//
//  ContentView.swift
//  pax-dev-harness-macos
//
//  Created by Zachary Brown on 4/6/22.
//

import SwiftUI

//struct ContentView: View {
//    var body: some View {
//        Text("Hello, world!")
//            .padding()
//    }
//}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}



struct ContentView: View {
   var body: some View {
      VStack {
         Text("Global Sales")
//         MyRepresentedCustomView()
      }
   }
}

// see: https://developer.apple.com/documentation/swiftui/nsviewrepresentable
