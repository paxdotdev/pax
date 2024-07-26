//
//  FlexBuffers.swift
//  FlexBuffers
//
//  Created by Maxim Zaks on 27.12.16.
//  Copyright © 2016 Maxim Zaks. All rights reserved.
//

import Foundation
import CoreGraphics

// MARK: FlexBuffers Builder logic


protocol WritableValueType {}

extension Int: WritableValueType {}
extension Int8: WritableValueType {}
extension Int16: WritableValueType {}
extension Int32: WritableValueType {}
extension Int64: WritableValueType {}

extension UInt: WritableValueType {}
extension UInt8: WritableValueType {}
extension UInt16: WritableValueType {}
extension UInt32: WritableValueType {}
extension UInt64: WritableValueType {}

extension Float32: WritableValueType {}
extension Float64: WritableValueType {}

extension Bool: WritableValueType {}

public class FlexBuffer {
    
    let initialSize : Int
    var currentSize : Int
    let options : BuilderOptions
    var finished = false
    var buffer : UnsafeMutableRawPointer
    var offset : Int
    fileprivate var stack : [Value] = []
    var keyPool : [String:Int] = [:]
    var stringPool : [String:Int] = [:]
    
    public init(initialSize : Int = 2048, options : BuilderOptions = []) {
        if initialSize <= 0 {
            self.initialSize = 2048
            self.currentSize = 2048
        } else {
            self.initialSize = initialSize
            self.currentSize = initialSize
        }
        
        self.options = options
        buffer = UnsafeMutableRawPointer.allocate(byteCount: self.initialSize, alignment: 1)
        buffer.initializeMemory(as: UInt8.self, repeating: 0, count: self.initialSize)
        offset = 0
    }
    
    deinit {
        buffer.deallocate()
    }
    
    public func addVector(_ f : () throws -> ()) throws {
        let start = startVector()
        try f()
        _ = try endVector(start: start, typed: false, fixed: false)
    }
    
    public func addVector(key : StaticString, _ f : () throws -> ()) throws {
        self.key(key)
        let start = startVector()
        try f()
        _ = try endVector(start: start, typed: false, fixed: false)
    }
    
    public func addVector(keyString : String, _ f : () throws -> ()) throws {
        self.key(keyString)
        let start = startVector()
        try f()
        _ = try endVector(start: start, typed: false, fixed: false)
    }
    
    public func addMap(_ f : () throws -> ()) throws {
        let start = startMap()
        try f()
        try endMap(start: start)
    }
    
    public func addMap(key : StaticString, _ f : () throws -> ()) throws {
        self.key(key)
        let start = startMap()
        try f()
        try endMap(start: start)
    }
    
    public func addMap(keyString : String, _ f : () throws -> ()) throws {
        self.key(keyString)
        let start = startMap()
        try f()
        try endMap(start: start)
    }
    
    fileprivate struct Value {
        enum Container {
            case int(Int64)
            case uint(UInt64)
            case double(Double)
            var asInt : Int64 {
                switch self {
                case let .int(value):
                    return Int64(value)
                case let .uint(value):
                    return Int64(value)
                case let .double(value):
                    return Int64(value)
                }
            }
            var asUInt : UInt64 {
                switch self {
                case let .int(value):
                    return UInt64(value)
                case let .uint(value):
                    return UInt64(value)
                case let .double(value):
                    return UInt64(value)
                }
            }
            var asDouble : Double {
                switch self {
                case let .int(value):
                    return Double(value)
                case let .uint(value):
                    return Double(value)
                case let .double(value):
                    return Double(value)
                }
            }
        }
        let value : Container
        let type : Type
        let minBitWidth : BitWidth
        
        init() {
            value = Value.Container.int(0)
            type = .null
            minBitWidth = .width8
        }
        
        init(value : Int64, type : Type, bitWidth : BitWidth) {
            self.value = Value.Container.int(value)
            self.type = type
            minBitWidth = bitWidth
        }
        
        init(value : UInt64, type : Type, bitWidth : BitWidth) {
            self.value = Value.Container.uint(value)
            self.type = type
            minBitWidth = bitWidth
        }
        
        init(value : Float) {
            self.value = Value.Container.double(Double(value))
            self.type = .float
            minBitWidth = .width32
        }
        
        init(value : Double) {
            self.value = Value.Container.double(value)
            self.type = .float
            minBitWidth = BitWidth.width(double: value)
        }
        
        init(value : Bool) {
            self.value = Value.Container.uint(value ? 1 : 0)
            self.type = .bool
            minBitWidth = BitWidth.width8
        }
        
        func storedWidth(bitWidth : BitWidth = .width8) -> BitWidth {
            if type.isInline {
                return BitWidth(rawValue: max(minBitWidth.rawValue, bitWidth.rawValue))!
            }
            return minBitWidth
        }
        
        func storedPackedType(width : BitWidth = .width8) -> UInt8 {
            return packedType(width: storedWidth(bitWidth: width), type: type)
        }
        
        func elementWidth(size : Int, index : Int) throws -> BitWidth {
            if type.isInline {
                return minBitWidth
            } else {
                for width in widths {
                    let offset_loc = size + paddingSize(bufSize: size, scalarSize: width) + index * Int(width)
                    let offset = offset_loc - Int(value.asInt)
                    let bit_width = BitWidth.width(int: Int64(offset))
                    if (1 << bit_width.rawValue) == UInt8(width) {
                        return bit_width
                    }
                }
                throw FlexBufferEncodeError(message: "Elelemtn with size: \(size) and index: \(index) is of unknown width")
            }
        }
    }
    
    fileprivate func null() {
        stack.append(Value())
    }
    
    fileprivate func int(_ i : Int64) {
        stack.append(Value(value: i, type: .int, bitWidth: BitWidth.width(int: i)))
    }
    
    fileprivate func int(_ i : Int) {
        stack.append(Value(value: Int64(i), type: .int, bitWidth: BitWidth.width(int: Int64(i))))
    }
    
    fileprivate func uint(_ i : UInt64) {
        stack.append(Value(value: i, type: .uint, bitWidth: BitWidth.width(uint: i)))
    }
    
    fileprivate func uint(_ i : UInt) {
        stack.append(Value(value: UInt64(i), type: .uint, bitWidth: BitWidth.width(uint: UInt64(i))))
    }
    
    fileprivate func float(_ f : Float) {
        stack.append(Value(value: f))
    }
    
    fileprivate func double(_ d : Double) {
        stack.append(Value(value: d))
    }
    
    fileprivate func bool(_ b : Bool) {
        stack.append(Value(value: b))
    }
    
    fileprivate func string(_ s : String) {
        let chars = s.utf8CString
        let length = chars.count - 1
        let bitWidth = BitWidth.width(uint: UInt64(length))
        if options.contains(.shareStrings), let sloc = stringPool[s] {
            stack.append(Value(value: UInt64(sloc), type: .string, bitWidth: bitWidth))
            return
        }
        let byteWidth = align(width: bitWidth)
        write(value: length, size: byteWidth)
        let sloc = offset
        for c in chars {
            write(value: c, size: 1)
        }
        stack.append(Value(value: UInt64(sloc), type: .string, bitWidth: bitWidth))
        if options.contains(.shareStrings) {
            stringPool[s] = sloc
        }
    }
    
    fileprivate func string(_ s : StaticString) {
        guard options.contains(.shareStrings) == false else {
            return string(s.description)
        }
        var sloc = offset
        let bitWidth = s.withUTF8Buffer { (buffer) -> BitWidth in
            let length = buffer.count
            let bitWidth = BitWidth.width(uint: UInt64(length))
            let byteWidth = align(width: bitWidth)
            write(value: length, size: byteWidth)
            sloc = offset
            for c in buffer {
                let ci = UInt8(c)
                write(value: ci, size: 1)
            }
            write(value: UInt8(0), size: 1)
            return bitWidth
        }
        stack.append(Value(value: UInt64(sloc), type: .string, bitWidth: bitWidth))
    }
    
    fileprivate func string(_ start : UnsafeMutablePointer<UInt8>, _ count : Int) {
        guard options.contains(.shareStrings) == false else {
            start.withMemoryRebound(to: CChar.self, capacity: 1) { ptr in
                if let s = NSString(bytes: UnsafeRawPointer(start), length: count, encoding: String.Encoding.utf8.rawValue) {
                    string(s as String)
                }
            }
            
            return
        }
        var sloc = offset
        
        let bitWidth = BitWidth.width(uint: UInt64(count))
        let byteWidth = align(width: bitWidth)
        write(value: count, size: byteWidth)
        sloc = offset
        for i in 0..<count {
            write(value: start.advanced(by: i).pointee, size: 1)
        }
        write(value: UInt8(0), size: 1)
        stack.append(Value(value: UInt64(sloc), type: .string, bitWidth: bitWidth))
    }
    
    func key(_ s : String) {
        let sloc : Int
        if options.contains(.shareKeys), let index = keyPool[s] {
            sloc = index
        } else {
            sloc = offset
            for c in s.utf8CString {
                write(value: c, size: 1)
            }
            if options.contains(.shareKeys) {
                keyPool[s] = sloc
            }
        }
        
        stack.append(Value(value: UInt64(sloc), type: .key, bitWidth: .width8))
    }
    
    func key(_ s : StaticString) {
        let sloc : Int
        if options.contains(.shareKeys) == false {
            sloc = offset
            s.withUTF8Buffer {
                for c in $0 {
                    write(value: c, size: 1)
                }
                write(value: UInt8(0), size: 1)
            }
        } else {
            let sd = s.description
            
            if let index = keyPool[sd] {
                sloc = index
            } else {
                sloc = offset
                s.withUTF8Buffer {
                    for c in $0 {
                        write(value: c, size: 1)
                    }
                    write(value: UInt8(0), size: 1)
                }
                keyPool[sd] = sloc
            }
        }
        stack.append(Value(value: UInt64(sloc), type: .key, bitWidth: .width8))
    }
    
    fileprivate func key(_ start : UnsafeMutablePointer<UInt8>, _ count : Int) {
        let sloc : Int
        if options.contains(.shareKeys) == false {
            sloc = offset
            for i in 0..<count {
                write(value: start.advanced(by: i).pointee, size: 1)
            }
            write(value: UInt8(0), size: 1)
        } else {
            let sd = String.init(bytesNoCopy: start, length: count, encoding: String.Encoding.utf8, freeWhenDone: false)
            
            if let sd = sd, let index = keyPool[sd] {
                sloc = index
            } else {
                sloc = offset
                
                for i in 0..<count {
                    write(value: start.advanced(by: i).pointee, size: 1)
                }
                write(value: UInt8(0), size: 1)
                if let sd = sd {
                    keyPool[sd] = sloc
                }
            }
        }
        stack.append(Value(value: UInt64(sloc), type: .key, bitWidth: .width8))
    }
    
    fileprivate func pushIndirect(_ val : Float) {
        let type = Type.indirect_float
        let bitWidth = BitWidth.width32
        let byteWitdth = align(width: bitWidth)
        let iloc = offset
        write(value: val, size: byteWitdth)
        stack.append(Value(value: UInt64(iloc), type: type, bitWidth: bitWidth))
    }
    
    fileprivate func pushIndirect(_ val : Double) throws {
        let type = Type.indirect_float
        let bitWidth = BitWidth.width(double: val)
        let byteWitdth = align(width: bitWidth)
        let iloc = offset
        try writeDouble(value: val, size: byteWitdth)
        stack.append(Value(value: UInt64(iloc), type: type, bitWidth: bitWidth))
    }
    
    fileprivate func pushIndirect(_ val : Int) {
        let type = Type.indirect_int
        let bitWidth = BitWidth.width(int: Int64(val))
        let byteWitdth = align(width: bitWidth)
        let iloc = offset
        write(value: val, size: byteWitdth)
        stack.append(Value(value: UInt64(iloc), type: type, bitWidth: bitWidth))
    }
    
    fileprivate func pushIndirect(_ val : Int64) {
        let type = Type.indirect_int
        let bitWidth = BitWidth.width(int: val)
        let byteWitdth = align(width: bitWidth)
        let iloc = offset
        write(value: val, size: byteWitdth)
        stack.append(Value(value: UInt64(iloc), type: type, bitWidth: bitWidth))
    }
    
    fileprivate func pushIndirect(_ val : UInt) {
        let type = Type.indirect_uint
        let bitWidth = BitWidth.width(uint: UInt64(val))
        let byteWitdth = align(width: bitWidth)
        let iloc = offset
        write(value: val, size: byteWitdth)
        stack.append(Value(value: UInt64(iloc), type: type, bitWidth: bitWidth))
    }
    
    fileprivate func pushIndirect(_ val : UInt64) {
        let type = Type.indirect_uint
        let bitWidth = BitWidth.width(uint: val)
        let byteWitdth = align(width: bitWidth)
        let iloc = offset
        write(value: val, size: byteWitdth)
        stack.append(Value(value: UInt64(iloc), type: type, bitWidth: bitWidth))
    }
    
    fileprivate func align(width : BitWidth) -> UInt8 {
        let byteWidth = 1 << width.rawValue
        offset += paddingSize(bufSize: offset, scalarSize: UInt8(byteWidth))
        return UInt8(byteWidth)
    }


    
    fileprivate func write<T: WritableValueType>(value : T, size : UInt8) {
        var v = value
        let newOffest = offset + Int(size)
        let prevSize = currentSize
        while currentSize < newOffest {
            currentSize = currentSize << 1
        }
        if prevSize < currentSize {
            let prevBuffer = buffer
            buffer = UnsafeMutableRawPointer.allocate(byteCount: currentSize, alignment: 1)
            buffer.initializeMemory(as: UInt8.self, repeating: 0, count: currentSize)
            buffer.copyMemory(from: prevBuffer, byteCount: offset)
            prevBuffer.deallocate()
        }
        buffer.advanced(by: offset).copyMemory(from: &v, byteCount: Int(size))
        offset = newOffest
    }
    
    fileprivate func writeOffset(value : UInt64, size : UInt8) throws {
        let reloff = UInt64(offset) - value
        guard size == 8 || reloff < UInt64(1 << UInt64(size * 8)) else {
            throw FlexBufferEncodeError(message: "Size is \(size) relative offset is \(reloff) which is bigger than \(UInt64(1 << UInt64(size * 8)))")
        }
        
        write(value: reloff, size: size)
    }
    
    fileprivate func writeDouble(value : Double, size : UInt8) throws {
        switch size {
        case 8:
            write(value: value, size: size)
        case 4:
            write(value: Float32(value), size: size)
        default:
            throw FlexBufferEncodeError(message: "Only 4 and 8 byte float numbers are supported. '\(value)' is float of size \(size)")
        }
    }
    
    private func write(flxvalue : Value, width : UInt8) throws {
        switch flxvalue.type {
        case .null:
            write(value: 0, size: width)
        case .int:
            write(value: flxvalue.value.asInt, size: width)
        case .uint:
            write(value: flxvalue.value.asUInt, size: width)
        case .bool:
            write(value: flxvalue.value.asUInt, size: width)
        case .float:
            try writeDouble(value: flxvalue.value.asDouble, size: width)
        default:
            try writeOffset(value: flxvalue.value.asUInt, size: width)
        }
    }
    
    func startVector() -> Int {
        return stack.count
    }
    
    func startMap() -> Int {
        return stack.count
    }
    
    func endVector(start : Int, typed : Bool, fixed : Bool) throws -> Int {
        let vec = try creteVector(start: start, vecLen: stack.count - start, step: 1, typed: typed, fixed: fixed)
        stack.removeLast(stack.count - start)
        stack.append(vec)
        return Int(vec.value.asUInt)
    }
    
    func endMap(start : Int) throws {
        var len = stack.count - start
        guard (len % 2) == 0 else {
            throw FlexBufferEncodeError(message: "We should have interleaved keys and values on the stack. Make sure it is an even number. Currently it is \(len)")
        }
        len /= 2
        
        var sorted = true
        for i in stride(from: start, to: stack.count, by: 2) {
            guard stack[i].type == .key else {
                throw FlexBufferEncodeError(message: "Make sure keys are all strings. Key at index \(i) is of type \(stack[i].type)")
            }
            guard i + 2 < stack.count else {
                break
            }
            if shouldFilp(stack[i], stack[i+2]) {
                sorted = false
                // don't break because of the assert
            }
        }
        if sorted == false {
            for i in stride(from: start, to: stack.count, by: 2) {
                // Now sort values, so later we can do a binary seach lookup.
                // We want to sort 2 array elements at a time.
                // performing selection sort (https://en.wikipedia.org/wiki/Selection_sort)
                guard i + 2 < stack.count else {
                    break
                }
                var flipIndex = i
                for i2 in stride(from: i + 2, to: stack.count, by: 2) {
                    if shouldFilp(stack[flipIndex], stack[i2]) {
                        flipIndex = i2
                    }
                }
                if flipIndex != i {
                    let k = stack[flipIndex]
                    let v = stack[flipIndex + 1]
                    stack[flipIndex] = stack[i]
                    stack[flipIndex + 1] = stack[i + 1]
                    stack[i] = k
                    stack[i + 1] = v
                }
            }
        }
        
        let keys = try creteVector(start: start, vecLen: len, step: 2, typed: true, fixed: false)
        let vec = try creteVector(start: start + 1, vecLen: len, step: 2, typed: false, fixed: false, keys: keys)
        
        stack.removeLast(stack.count - start)
        stack.append(vec)
        
    }
    
    private func shouldFilp(_ v1 : Value, _ v2 : Value) -> Bool {
        var index = 0
        var c1 : UInt8
        var c2 : UInt8
        
        
        repeat {
            c1 = buffer.load(fromByteOffset: Int(v1.value.asInt + Int64(index)), as: UInt8.self)//buffer[Int(v1.value.asInt + index)]
            c2 = buffer.load(fromByteOffset: Int(v2.value.asInt + Int64(index)), as: UInt8.self)//buffer[Int(v2.value.asInt + index)]
            if c2 < c1 {
                return true
            } else if c1 < c2 {
                return false
            }
            index += 1
        } while c1 != 0 && c2 != 0
        return false
    }
    
    fileprivate func creteVector(start : Int, vecLen : Int, step : Int, typed : Bool,
                     fixed: Bool,
                     keys : Value? = nil) throws -> Value {
        // Figure out smallest bit width we can store this vector with
        var bitWidth = BitWidth.width(uint: UInt64(vecLen))
        var prefixElems = 1
        
        if let keys = keys {
            // If this vector is part of a map, we will pre-fix an offset to the keys
            // to this vector.
            let elemWidth = try keys.elementWidth(size: offset, index: 0)
            guard let _bitWidth = BitWidth(rawValue: max(bitWidth.rawValue, elemWidth.rawValue)) else {
                throw FlexBufferEncodeError(message: "\(max(bitWidth.rawValue, elemWidth.rawValue)) is an unknown bit width type")
            }
            bitWidth = _bitWidth
            prefixElems += 2
        }
        
        var vectorType = Type.key
        for i in stride(from: start, to: stack.count, by: step) {
            let elemWidth = try stack[i].elementWidth(size: offset, index: i + prefixElems)
            guard let _bitWidth = BitWidth(rawValue: max(bitWidth.rawValue, elemWidth.rawValue)) else {
                throw FlexBufferEncodeError(message: "\(max(bitWidth.rawValue, elemWidth.rawValue)) is an unknown bit width type")
            }
            bitWidth = _bitWidth
            if typed {
                if i == start {
                    vectorType = stack[i].type
                } else {
                    guard vectorType == stack[i].type else {
                        throw FlexBufferEncodeError(message: "Your typed vector is of type \(vectorType) but the item on index \(i) is of type \(stack[i].type)")
                    }
                }
            }
        }
        guard vectorType.isTypedVectorElement else {
            throw FlexBufferEncodeError(message: "Your fixed types are not one of: Int / UInt / Float / Key")
        }
        let byteWidth = align(width: bitWidth)
        // Write vector. First the keys width/offset if available, and size.
        if let keys = keys {
            try writeOffset(value: keys.value.asUInt, size: byteWidth)
            write(value: UInt64(1 << keys.minBitWidth.rawValue), size: byteWidth)
        }
        if !fixed {
            write(value: vecLen, size: byteWidth)
        }
        // Then the actual data.
        let vloc = offset
        for i in stride(from: start, to: stack.count, by: step) {
            try write(flxvalue: stack[i], width: byteWidth)
        }
        // Then the types.
        if !typed {
            for i in stride(from: start, to: stack.count, by: step) {
                write(value: stack[i].storedPackedType(width: bitWidth), size: 1)
            }
        }
        
        return Value(value: UInt64(vloc),
                     type: keys != nil
                            ? .map
                            : (typed
                                ? try vectorType.toTypedVector(length: UInt8(fixed ? vecLen : 0))
                                : .vector),
                     bitWidth: bitWidth)
    }
    
    public func finish() throws -> Data {
        
        if !finished {
            try finishBuffer()
        }
        
        let data = Data(bytes:buffer, count: offset)
        offset = 0
        if options.contains(.shareKeys){
            keyPool.removeAll(keepingCapacity: true)
        }
        
        return data
    }
    
    private func finishBuffer() throws {
        guard stack.count == 1 else {
            throw FlexBufferEncodeError(message: "You have \(stack.count) objects on stack instead of 1. You likely have objects that were never included in a parent. You need to have exactly one root to finish a buffer. Check your Start/End calls are matched, and all objects are inside some other object.")
        }
        
        let byteWidth = align(width: try stack[0].elementWidth(size: offset, index: 0))
        try write(flxvalue: stack[0], width: byteWidth)
        write(value: stack[0].storedPackedType(), size: 1)
        write(value: byteWidth, size: 1)
        finished = true
    }
}

extension FlexBuffer {
    
    public static func encode(_ v: FlxbValue) throws -> FlxbData {
        let builder = FlexBuffer()
        try builder.handleValue(v)
        let flxbData = try builder.finish()
        return FlxbData(data: flxbData)
    }
    
    public static var valueHandler : ((FlexBuffer, FlxbValue) throws -> Bool)?
    
    fileprivate func handleValue(_ v: FlxbValue) throws {
        if try FlexBuffer.valueHandler?(self, v) == true {
            return
        }
        if let i = v as? Int {
            int(i)
        } else if let u = v as? Int64 {
            int(u)
        } else if let u = v as? UInt {
            uint(u)
        } else if let u = v as? UInt64 {
            uint(u)
        } else if let b = v as? Bool {
            bool(b)
        } else if let d = v as? Double {
            double(d)
        } else if let s = v as? StaticString {
            string(s)
        } else if let s = v as? String {
            string(s)
        } else if let m = v as? FlxbValueMap {
            try addValueMap(m)
        } else if let v = v as? FlxbValueVector {
            try addValueVector(v)
        } else if let _ = v as? FlxbValueNil {
            addNull()
        } else if let p = v as? CGPoint {
            try add(value: (Double(p.x), Double(p.y)))
        } else if let s = v as? CGSize {
            try add(value: (Double(s.width), Double(s.height)))
        } else if let r = v as? CGRect {
            try add(value: (Double(r.origin.x), Double(r.origin.y), Double(r.size.width), Double(r.size.height)))
        }  else if let d = v as? Data {
            add(value: d)
        } else {
            throw FlexBufferEncodeError(message: "Unexpected FlxbValue type added `\(type(of: v))`. Consider setting your own `FlexBuffer.valueHandler`")
        }
    }
    fileprivate func addValueVector(_ array : FlxbValueVector) throws {
        let start = startVector()
        for v in array.values {
            try handleValue(v)
        }
        _ = try endVector(start: start, typed: false, fixed: false)
    }
    
    fileprivate func addValueMap(_ dict : FlxbValueMap) throws {
        let start = startMap()
        for v in dict.values {
            self.key(v.0)
            try handleValue(v.1)
        }
        try endMap(start: start)
    }
}

public protocol FlxbValue {}
extension Int: FlxbValue {}
extension Int64: FlxbValue {}
extension UInt: FlxbValue {}
extension UInt64: FlxbValue {}
extension Bool: FlxbValue {}
extension Double: FlxbValue {}
extension StaticString: FlxbValue {}
extension String: FlxbValue {}
extension CGPoint: FlxbValue {}
extension CGRect: FlxbValue {}
extension CGSize: FlxbValue {}
extension Data: FlxbValue {}

public struct FlxbValueNil: FlxbValue {
    public init(){}
}

public struct FlxbValueVector: FlxbValue, ExpressibleByArrayLiteral {
    let values: [FlxbValue]
    public init(arrayLiteral elements: FlxbValue...) {
        values = elements
    }
    public init(values: [FlxbValue]) {
        self.values = values
    }
}

public struct FlxbValueMap: FlxbValue, ExpressibleByDictionaryLiteral {
    let values: [(StaticString, FlxbValue)]
    public init(dictionaryLiteral elements: (StaticString, FlxbValue)...) {
        values = elements
    }
}

// MARK: - DECODER

extension FlexBuffer {
    public static func decode(data: Data) -> FlxbReference? {
        guard data.count > 2 else {
            return nil
        }
        let byteWidth = data[data.count - 1]
        let packedType = data[data.count - 2]

        let p2 = UnsafeMutableRawPointer.allocate(byteCount: data.count, alignment: 1)

        data.withUnsafeBytes { (p) in
            p2.copyMemory(from: p.baseAddress!, byteCount: data.count)
        }
        let p = p2.advanced(by: (data.count - Int(byteWidth) - 2))
        return FlxbReference(dataPointer: UnsafeRawPointer(p), parentWidth: byteWidth, packedType: packedType)
    }
}

// MARK: - addMethod for Primitives

public extension FlexBuffer {
    func addNull(){
        null()
    }
    func addNull(keyString : String){
        self.key(keyString)
        null()
    }
    func addNull(key : StaticString){
        self.key(key)
        null()
    }
    func add(value v : Bool){
        bool(v)
    }
    func add(keyString : String, value v : Bool){
        self.key(keyString)
        bool(v)
    }
    func add(key : StaticString, value v : Bool){
        self.key(key)
        bool(v)
    }
    func add(array vs : [Bool]) throws {
        let start = startVector()
        for v in vs {
            bool(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [Bool]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [Bool]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : Int){
        int(v)
    }
    func add(keyString : String, value v : Int){
        self.key(keyString)
        int(v)
    }
    func add(key : StaticString, value v : Int){
        self.key(key)
        int(v)
    }
    func add(indirectValue v : Int){
        pushIndirect(v)
    }
    func add(keyString : String, indirectValue v : Int){
        self.key(keyString)
        pushIndirect(v)
    }
    func add(key : StaticString, indirectValue v : Int){
        self.key(key)
        pushIndirect(v)
    }
    func add(array vs : [Int]) throws {
        let start = startVector()
        for v in vs {
            int(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [Int]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [Int]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : Int64){
        int(v)
    }
    func add(keyString : String, value v : Int64){
        self.key(keyString)
        int(v)
    }
    func add(key : StaticString, value v : Int64){
        self.key(key)
        int(v)
    }
    func add(indirectValue v : Int64){
        pushIndirect(v)
    }
    func add(keyString : String, indirectValue v : Int64){
        self.key(keyString)
        pushIndirect(v)
    }
    func add(key : StaticString, indirectValue v : Int64){
        self.key(key)
        pushIndirect(v)
    }
    func add(array vs : [Int64]) throws {
        let start = startVector()
        for v in vs {
            int(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [Int64]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [Int64]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : UInt){
        uint(v)
    }
    func add(keyString : String, value v : UInt){
        self.key(keyString)
        uint(v)
    }
    func add(key : StaticString, value v : UInt){
        self.key(key)
        uint(v)
    }
    func add(indirectValue v : UInt){
        pushIndirect(v)
    }
    func add(keyString : String, indirectValue v : UInt){
        self.key(keyString)
        pushIndirect(v)
    }
    func add(key : StaticString, indirectValue v : UInt){
        self.key(key)
        pushIndirect(v)
    }
    func add(array vs : [UInt]) throws {
        let start = startVector()
        for v in vs {
            uint(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [UInt]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [UInt]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : UInt64){
        uint(v)
    }
    func add(keyString : String, value v : UInt64){
        self.key(keyString)
        uint(v)
    }
    func add(key : StaticString, value v : UInt64){
        self.key(key)
        uint(v)
    }
    func add(indirectValue v : UInt64){
        pushIndirect(v)
    }
    func add(keyString : String, indirectValue v : UInt64){
        self.key(keyString)
        pushIndirect(v)
    }
    func add(key : StaticString, indirectValue v : UInt64){
        self.key(key)
        pushIndirect(v)
    }
    func add(array vs : [UInt64]) throws {
        let start = startVector()
        for v in vs {
            uint(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [UInt64]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [UInt64]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : Float){
        float(v)
    }
    func add(keyString : String, value v : Float){
        self.key(keyString)
        float(v)
    }
    func add(key : StaticString, value v : Float){
        self.key(key)
        float(v)
    }
    func add(indirectValue v : Float){
        pushIndirect(v)
    }
    func add(keyString : String, indirectValue v : Float){
        self.key(keyString)
        pushIndirect(v)
    }
    func add(key : StaticString, indirectValue v : Float){
        self.key(key)
        pushIndirect(v)
    }
    func add(array vs : [Float]) throws {
        let start = startVector()
        for v in vs {
            float(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [Float]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [Float]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : Double){
        double(v)
    }
    func add(keyString : String, value v : Double){
        self.key(keyString)
        double(v)
    }
    func add(key : StaticString, value v : Double){
        self.key(key)
        double(v)
    }
    func add(indirectValue v : Double) throws {
        try pushIndirect(v)
    }
    func add(keyString : String, indirectValue v : Double) throws {
        self.key(keyString)
        try pushIndirect(v)
    }
    func add(key : StaticString, indirectValue v : Double) throws {
        self.key(key)
        try pushIndirect(v)
    }
    func add(array vs : [Double]) throws {
        let start = startVector()
        for v in vs {
            double(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [Double]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [Double]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(stringValue v : String){
        string(v)
    }
    func add(keyString : String, stringValue v : String){
        self.key(keyString)
        string(v)
    }
    func add(key : StaticString, stringValue v : String){
        self.key(key)
        string(v)
    }
    func add(array vs : [String]) throws {
        let start = startVector()
        for v in vs {
            string(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [String]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [String]) throws {
        self.key(key)
        try add(array:vs)
    }
    func add(value v : StaticString){
        string(v)
    }
    func add(keyString : String, value v : StaticString){
        self.key(keyString)
        string(v)
    }
    func add(key : StaticString, value v : StaticString){
        self.key(key)
        string(v)
    }
    func add(array vs : [StaticString]) throws {
        let start = startVector()
        for v in vs {
            string(v)
        }
        _ = try endVector(start: start, typed: true, fixed: false)
    }
    func add(keyString : String, value vs : [StaticString]) throws {
        self.key(keyString)
        try add(array:vs)
    }
    func add(key : StaticString, value vs : [StaticString]) throws {
        self.key(key)
        try add(array:vs)
    }
    
    func add(value: Data){
        let length = value.count
        let bitWidth = BitWidth.width(uint: UInt64(length))
        let byteWidth = align(width: bitWidth)
        write(value: length, size: byteWidth)
        let sloc = offset

        _ = value.withUnsafeBytes { (byte) -> Bool in
            for i in 0..<length {
                let valueToWrite = byte.baseAddress?.assumingMemoryBound(to: UInt8.self).advanced(by: i).pointee ?? 0
                write(value: valueToWrite, size: 1)
            }
            return true
        }
        
        stack.append(Value(value: UInt64(sloc), type: .blob, bitWidth: bitWidth))
    }
    func add(keyString : String, value v : Data) throws {
        self.key(keyString)
        add(value:v)
    }
    func add(key : StaticString, value v : Data) throws {
        self.key(key)
        add(value:v)
    }
    
// MARK: - TUPPLES
    
    func add(value v : (Int, Int)) throws {
        let start = startVector()
        int(v.0)
        int(v.1)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (Int, Int)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (Int, Int)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (UInt, UInt)) throws {
        let start = startVector()
        uint(v.0)
        uint(v.1)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (UInt, UInt)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (UInt, UInt)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (Double, Double)) throws {
        let start = startVector()
        double(v.0)
        double(v.1)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (Double, Double)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (Double, Double)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (Int, Int, Int)) throws {
        let start = startVector()
        int(v.0)
        int(v.1)
        int(v.2)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (Int, Int, Int)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (Int, Int, Int)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (UInt, UInt, UInt)) throws {
        let start = startVector()
        uint(v.0)
        uint(v.1)
        uint(v.2)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (UInt, UInt, UInt)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (UInt, UInt, UInt)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (Double, Double, Double)) throws {
        let start = startVector()
        double(v.0)
        double(v.1)
        double(v.2)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (Double, Double, Double)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (Double, Double, Double)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (Int, Int, Int, Int)) throws {
        let start = startVector()
        int(v.0)
        int(v.1)
        int(v.2)
        int(v.3)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (Int, Int, Int, Int)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (Int, Int, Int, Int)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (UInt, UInt, UInt, UInt)) throws {
        let start = startVector()
        uint(v.0)
        uint(v.1)
        uint(v.2)
        uint(v.3)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (UInt, UInt, UInt, UInt)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (UInt, UInt, UInt, UInt)) throws {
        self.key(key)
        try add(value:vs)
    }
    
    func add(value v : (Double, Double, Double, Double)) throws {
        let start = startVector()
        double(v.0)
        double(v.1)
        double(v.2)
        double(v.3)
        _ = try endVector(start: start, typed: true, fixed: true)
    }
    func add(keyString : String, value vs : (Double, Double, Double, Double)) throws {
        self.key(keyString)
        try add(value:vs)
    }
    func add(key: StaticString, value vs : (Double, Double, Double, Double)) throws {
        self.key(key)
        try add(value:vs)
    }
}

// MARK: - ACCESSORS

public struct FlxbData : CustomDebugStringConvertible {
    public let data: Data
    public let root: FlxbReference?
    public init(data: Data) {
        self.data = data
        self.root = FlexBuffer.decode(data: data)
    }
    public var debugDescription: String {
        return self.root?.debugDescription ?? "\(data)"
    }
}

public struct FlxbReference : CustomDebugStringConvertible {
    fileprivate let dataPointer : UnsafeRawPointer
    fileprivate let parentWidth : UInt8
    fileprivate let byteWidth : UInt8
    fileprivate let type : Type
    
    fileprivate init?(dataPointer : UnsafeRawPointer, parentWidth : UInt8, packedType : UInt8) {
        self.dataPointer = dataPointer
        self.parentWidth = parentWidth
        guard let byteWidth = BitWidth(rawValue: packedType & 3)?.rawValue,
            let type = Type(rawValue: packedType >> 2) else {
                return nil
        }
        self.byteWidth = 1 << byteWidth
        self.type = type
    }
    
    fileprivate init(dataPointer : UnsafeRawPointer, parentWidth : UInt8, byteWidth : UInt8, type : Type){
        self.dataPointer = dataPointer
        self.parentWidth = parentWidth
        self.byteWidth = byteWidth
        self.type = type
    }
    
    public subscript(index: Int) -> FlxbReference? {
        guard let vector = self.asVector else {
            return nil
        }
        return vector[index]
    }
    
    public subscript(key: StaticString) -> FlxbReference? {
        guard let map = self.asMap else {
            return nil
        }
        return map[key]
    }
    
    public func get(key: String) -> FlxbReference? {
        guard let map = self.asMap else {
            return nil
        }
        return map.get(key: key)
    }
    
    public var isNull : Bool {
        return type == .null
    }
    
    public var asInt : Int? {
        guard let r = asInt64 else {
            return nil
        }
        return Int(r)
    }
    
    public var asInt64 : Int64? {
        switch type {
        case .int :
            return readInt(pointer: dataPointer, width: parentWidth)
        case .indirect_int:
            if let p = self.indirect {
                return readInt(pointer: p, width: byteWidth)
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asUInt : UInt? {
        guard let r = asUInt64 else {
            return nil
        }
        return UInt(r)
    }
    
    public var asUInt64 : UInt64? {
        switch type {
        case .uint :
            return readUInt(pointer: dataPointer, width: parentWidth)
        case .indirect_uint :
            if let p = self.indirect {
                return readUInt(pointer: p, width: byteWidth)
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asFloat : Float? {
        switch type {
        case .float :
            return readFloat(pointer: dataPointer, width: parentWidth)
        case .indirect_float :
            if byteWidth == 8 {
                return nil
            }
            if let p = self.indirect {
                return readFloat(pointer: p, width: byteWidth)
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asDouble : Double? {
        switch type {
        case .float :
            return readDouble(pointer: dataPointer, width: parentWidth)
        case .indirect_float :
            if let p = self.indirect {
                return readDouble(pointer: p, width: byteWidth)
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asBool : Bool? {
        switch type {
        case .bool :
            let r = readUInt(pointer: dataPointer, width: parentWidth)
            if r == 0 {
                return false
            }
            if r == 1 {
                return true
            }
            return nil
        case .uint :
            let r = readUInt(pointer: dataPointer, width: parentWidth)
            if r == 0 {
                return false
            }
            if r == 1 {
                return true
            }
            return nil
        case .int :
            let r = readInt(pointer: dataPointer, width: parentWidth)
            if r == 0 {
                return false
            }
            if r == 1 {
                return true
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asString : String? {
        switch type {
        case .string :
            if let p = self.indirect {
                return FlxbString(dataPointer: p, byteWidth: byteWidth).string
            }
            return nil
        case .key :
            if let p = self.indirect {
                return String(validatingUTF8: p.assumingMemoryBound(to: CChar.self))
            }
            return nil
            
        default:
            return nil
        }
    }
    
    public var count : Int? {
        if type.isTypedVector {
            return asVector?.count
        }
        switch type {
        case .string :
            if let p = self.indirect {
                return FlxbString(dataPointer: p, byteWidth: byteWidth).count
            }
            return nil
        case .vector :
            return asVector?.count
        case .map :
            return asMap?.count
        default:
            return nil
        }
    }
    
    fileprivate var asPointer : UnsafeRawPointer? {
        switch type {
        case .key :
            return self.indirect
        default:
            return nil
        }
    }
    
    public var asVector : FlxbVector? {
        if type.isTypedVector {
            if let p = self.indirect {
                return FlxbVector(dataPointer: p, byteWidth: byteWidth, type: type.typedVectorElementType)
            }
            return nil
        }
        if type.isFixedTypedVector {
            if let p = self.indirect {
                let (type, length) = self.type.fixedTypedVectorElementType
                return FlxbVector(dataPointer: p, byteWidth: byteWidth, type: type, length: length)
            }
        }
        switch type {
        case .vector :
            if let p = self.indirect {
                return FlxbVector(dataPointer: p, byteWidth: byteWidth)
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asMap : FlxbMap? {
        switch type {
        case .map :
            if let p = self.indirect {
                return FlxbMap(dataPointer: p, byteWidth: byteWidth)
            }
            return nil
        default:
            return nil
        }
    }
    
    public var asTuppleInt : (Int, Int)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asInt,
                let v2 = v[1]?.asInt {
                return (v1, v2)
            }
        }
        return nil
    }
    
    public var asTuppleUInt : (UInt, UInt)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asUInt,
                let v2 = v[1]?.asUInt {
                return (v1, v2)
            }
        }
        return nil
    }
    
    public var asTuppleDouble : (Double, Double)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asDouble,
                let v2 = v[1]?.asDouble {
                return (v1, v2)
            }
        }
        return nil
    }
    
    public var asTripleInt : (Int, Int, Int)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asInt,
                let v2 = v[1]?.asInt,
                let v3 = v[2]?.asInt {
                return (v1, v2, v3)
            }
        }
        return nil
    }
    
    public var asTripleUInt : (UInt, UInt, UInt)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asUInt,
                let v2 = v[1]?.asUInt,
                let v3 = v[2]?.asUInt {
                return (v1, v2, v3)
            }
        }
        return nil
    }
    
    public var asTripleDouble : (Double, Double, Double)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asDouble,
                let v2 = v[1]?.asDouble,
                let v3 = v[2]?.asDouble {
                return (v1, v2, v3)
            }
        }
        return nil
    }
    
    public var asQuadrupleInt : (Int, Int, Int, Int)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asInt,
                let v2 = v[1]?.asInt,
                let v3 = v[2]?.asInt,
                let v4 = v[3]?.asInt {
                return (v1, v2, v3, v4)
            }
        }
        return nil
    }
    
    public var asQuadrupleUInt : (UInt, UInt, UInt, UInt)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asUInt,
                let v2 = v[1]?.asUInt,
                let v3 = v[2]?.asUInt,
                let v4 = v[3]?.asUInt {
                return (v1, v2, v3, v4)
            }
        }
        return nil
    }
    
    public var asQuadrupleDouble : (Double, Double, Double, Double)? {
        if type.isFixedTypedVector {
            if let v = self.asVector,
                let v1 = v[0]?.asDouble,
                let v2 = v[1]?.asDouble,
                let v3 = v[2]?.asDouble,
                let v4 = v[3]?.asDouble {
                return (v1, v2, v3, v4)
            }
        }
        return nil
    }
    
    public var asData: Data? {
        if type == .blob {
            if let p = self.indirect,
                let size = readUInt(pointer: p - Int(byteWidth), width: byteWidth) {
                return Data(bytes: p, count: Int(size))
            }
        }
        return nil
    }
    
    private var indirect : UnsafeRawPointer? {
        return _indirect(pointer: dataPointer, width: parentWidth)
    }
    
    public var debugDescription: String {
        if let v = asInt {
            return "\(v)"
        }
        if let v = asUInt {
            return "\(v)"
        }
        if let v = asFloat {
            return "\(v)"
        }
        if let v = asDouble {
            return "\(v)"
        }
        if let v = asBool {
            return v ? "true" : "false"
        }
        if let v = asString {
            let escaped_v = v.replacingOccurrences(of: "\"", with: "\\\"")
                                .replacingOccurrences(of: "\n", with: "\\n")
                                .replacingOccurrences(of: "\r", with: "\\r")
                                .replacingOccurrences(of: "\t", with: "\\t")
            return "\"\(escaped_v)\""
        }
        if let v = asMap {
            return v.debugDescription
        }
        if let v = asVector {
            return v.debugDescription
        }
        if let v = asData {
            return "\"\(v.base64EncodedString())\""
        }
        return "null"
    }
    
    public var jsonString: String {
        return debugDescription.replacingOccurrences(of: "/", with: "\\/")
    }
}

public struct FlxbString {
    fileprivate let dataPointer : UnsafeRawPointer
    fileprivate let byteWidth : UInt8
    var count : Int {
        if let size = readUInt(pointer: dataPointer - Int(byteWidth), width: byteWidth) {
            return Int(size)
        }
        return 0
    }
    
    var string : String? {
        return String(validatingUTF8: dataPointer.assumingMemoryBound(to: CChar.self))
    }
}

public struct FlxbVector : Sequence, CustomDebugStringConvertible {
    fileprivate let dataPointer : UnsafeRawPointer
    fileprivate let byteWidth : UInt8
    fileprivate let type : Type?
    fileprivate let length: UInt8?
    
    fileprivate init(dataPointer : UnsafeRawPointer, byteWidth : UInt8, type : Type? = nil, length: UInt8? = nil) {
        self.dataPointer = dataPointer
        self.byteWidth = byteWidth
        self.type = type
        self.length = length
    }
    
    public var count : Int {
        if let length = length {
            return Int(length)
        }
        if let size = readUInt(pointer: dataPointer - Int(byteWidth), width: byteWidth) {
            return Int(size)
        }
        return 0
    }
    
    public subscript(index: Int) -> FlxbReference? {
        let length = count
        guard index >= 0 && index < length else {
            return nil
        }
        if let type = type {
            return get(index, length, type)
        } else {
            return get(index, length)
        }
    }
    
    private func get(_ index : Int, _ length : Int) -> FlxbReference? {
        let packedType = (dataPointer + (length * Int(byteWidth))).load(fromByteOffset: index, as: UInt8.self)
        let elem = dataPointer + (index * Int(byteWidth))
        return FlxbReference(dataPointer: elem, parentWidth: byteWidth, packedType: packedType)
    }
    
    private func get(_ index : Int, _ length : Int, _ type : Type) -> FlxbReference? {
        let elem = dataPointer + (index * Int(byteWidth))
        return FlxbReference(dataPointer: elem, parentWidth: byteWidth, byteWidth: 1, type: type)
    }
    
    public func makeIterator() -> AnyIterator<FlxbReference> {
        
        var nextIndex = 0
        
        return AnyIterator<FlxbReference> {
            if(self.count <= nextIndex){
                return nil
            }
            let value = self[nextIndex]
            nextIndex += 1
            return value
        }
    }
    
    public var debugDescription: String {
        var values : [String] = []
        for i in 0..<count {
            if let v = self[i] {
                values.append(v.debugDescription)
            } else {
                values.append("null")
            }
        }
        let list = values.joined(separator: ",")
        return "[\(list)]"
    }
}

public struct FlxbMap : Sequence, CustomDebugStringConvertible {
    fileprivate let dataPointer : UnsafeRawPointer
    fileprivate let byteWidth : UInt8
    
    public var count : Int {
        if let size = readUInt(pointer: dataPointer - Int(byteWidth), width: byteWidth) {
            return Int(size)
        }
        return 0
    }
    
    public subscript(key: StaticString) -> FlxbReference? {
        guard let index = keyIndex(key: key) else {
            return nil
        }
        return values?[index]
    }
    
    public func get(key: String) -> FlxbReference? {
        guard let index = keyIndex(key: key) else {
            return nil
        }
        return values?[index]
    }
    
    private var keys : FlxbVector? {
        let keysOffset = dataPointer - Int(byteWidth) * 3
        if let p = _indirect(pointer: keysOffset, width: byteWidth),
            let bWidth = readUInt(pointer: keysOffset + Int(byteWidth), width: byteWidth) {
            return FlxbVector(dataPointer: p, byteWidth: UInt8(bWidth), type: .key)
        }
        return nil
    }
    
    private var values : FlxbVector? {
        return FlxbVector(dataPointer: dataPointer, byteWidth: byteWidth)
    }
    
    private func keyIndex(key : String) -> Int? {
        
        let key1 = key.utf8CString
        guard let _keys = keys else {
            return nil
        }
        
        func comp(i : Int) -> Int8? {
            guard let key2 = _keys[i]?.asPointer else {
                return nil
            }
            var index = 0
            
            while true {
                let c1 = key1[index]
                let c2 = key2.load(fromByteOffset: index, as: CChar.self)
                let c = c1 - c2
                if c != 0 {
                    return c
                }
                if c1 == 0 && c2 == 0 {
                    return 0
                }
                index += 1
            }
        }
        
        var low = 0
        var high = _keys.count - 1
        while low <= high {
            let mid = (high + low) >> 1
            guard let dif = comp(i: mid) else {
                return nil
            }
            if dif == 0 {
                return mid
            }
            if dif < 0 {
                high = mid - 1
            } else {
                low = mid + 1
            }
        }
        return nil
        
    }
    
    private func keyIndex(key : StaticString) -> Int? {
        
        let key1 = key.utf8Start
        guard let _keys = keys else {
            return nil
        }
        
        func comp(i : Int) -> Int8? {
            guard let key2 = _keys[i]?.asPointer else {
                return nil
            }
            var index = 0
            
            while true {
                let c1 = CChar(key1.advanced(by: index).pointee)
                let c2 = key2.load(fromByteOffset: index, as: CChar.self)
                let c = c1 &- c2
                
                if c != 0 {
                    return c
                }
                if c1 == 0 && c2 == 0 {
                    return 0
                }
                index += 1
            }
        }
        
        var low = 0
        var high = _keys.count - 1
        while low <= high {
            let mid = (high + low) >> 1
            guard let dif = comp(i: mid) else {
                return nil
            }
            if dif == 0 {
                return mid
            }
            if dif < 0 {
                high = mid - 1
            } else {
                low = mid + 1
            }
        }
        return nil
        
    }
    
    public func makeIterator() -> AnyIterator<(key : String, value: FlxbReference)> {
        
        var nextIndex = 0
        let _keys = keys
        let _values = values
        
        return AnyIterator<(key : String, value: FlxbReference)> {
            if(self.count <= nextIndex){
                return nil
            }
            if let key = _keys?[nextIndex]?.asString,
                let value = _values?[nextIndex] {
                let value = (key: key, value: value)
                nextIndex += 1
                return value
            }
            return nil
        }
    }
    
    public func toDict<T>(converter: (FlxbReference) -> T?) -> [String: T] {
        var result =  [String: T](minimumCapacity: self.count)
        for i in 0 ..< self.count {
            if let key = keys?[i]?.asString,
                let value = values?[i],
                let convertedValue = converter(value) {
                result[key] = convertedValue
            }
        }
        return result
    }
    
    public var debugDescription: String {
        if let keys = keys?.map({$0.debugDescription}), let values = values?.map({$0.debugDescription}) {
            let pairs = zip(keys, values)
            let list = pairs.map({"\($0):\($1)"})
            return "{\(list.joined(separator: ","))}"
        }
        return "{\(keys.debugDescription):\(values.debugDescription)}"
    }
}

// MARK: - Custom private Types and Functions

fileprivate func readInt(pointer : UnsafeRawPointer, width : UInt8) -> Int64? {
    guard 0 == (UInt(bitPattern: pointer) & (UInt(width) - 1)) else {
        return nil
    }
    if width == 1 {
        return Int64(pointer.load(as: Int8.self))
    }
    if width == 2 {
        return Int64(pointer.load(as: Int16.self))
    }
    if width == 4 {
        return Int64(pointer.load(as: Int32.self))
    }
    if width == 8 {
        return Int64(pointer.load(as: Int64.self))
    }
    return nil
}

fileprivate func readUInt(pointer : UnsafeRawPointer, width : UInt8) -> UInt64? {
    guard 0 == (UInt(bitPattern: pointer) & (UInt(width) - 1)) else {
        return nil
    }
    if width == 1 {
        return UInt64(pointer.load(as: UInt8.self))
    }
    if width == 2 {
        return UInt64(pointer.load(as: UInt16.self))
    }
    if width == 4 {
        return UInt64(pointer.load(as: UInt32.self))
    }
    if width == 8 {
        return UInt64(pointer.load(as: UInt64.self))
    }
    return nil
}

fileprivate func readFloat(pointer : UnsafeRawPointer, width : UInt8) -> Float? {
    guard 0 == (UInt(bitPattern: pointer) & (UInt(width) - 1)) else {
        return nil
    }
    if width == 4 {
        return pointer.load(as: Float.self)
    }
    if width == 8 {
        return Float(pointer.load(as: Double.self))
    }
    return nil
}

fileprivate func readDouble(pointer : UnsafeRawPointer, width : UInt8) -> Double? {
    guard 0 == (UInt(bitPattern: pointer) & (UInt(width) - 1)) else {
        return nil
    }
    if width == 4 {
        return Double(pointer.load(as: Float.self))
    }
    if width == 8 {
        return pointer.load(as: Double.self)
    }
    return nil
}

fileprivate func _indirect(pointer : UnsafeRawPointer, width : UInt8) -> UnsafeRawPointer? {
    guard let step = readUInt(pointer: pointer, width: width) else {
        return nil
    }
    return pointer - Int(step)
}

fileprivate enum BitWidth : UInt8 {
    case width8, width16, width32, width64
    static func width(uint : UInt64) -> BitWidth {
        if (uint & 0xFFFFFFFFFFFFFF00) == 0 {
            return .width8
        }
        if (uint & 0xFFFFFFFFFFFF0000) == 0 {
            return .width16
        }
        if (uint & 0xFFFFFFFF00000000) == 0 {
            return .width32
        }
        return .width64
    }
    
    static func width(int : Int64) -> BitWidth {
        let u = UInt64(bitPattern: int) << 1
        return width(uint: int >= 0 ? u : ~u)
    }
    static func width(double : Double) -> BitWidth {
        if Double(Float32(double)) == double {
            return .width32
        }
        return width64
    }
}

fileprivate enum Type : UInt8 {
    case null, int, uint, float,
    key, string, indirect_int, indirect_uint, indirect_float,
    map, vector, vector_int, vector_uint, vector_float, vector_key, vector_string,
    vector_int2, vector_uint2, vector_float2,
    vector_int3, vector_uint3, vector_float3,
    vector_int4, vector_uint4, vector_float4,
    blob, bool, vector_bool = 36
    
    var isInline : Bool {
        return self == .bool || self.rawValue <= Type.float.rawValue
    }
    
    var isTypedVectorElement : Bool {
        return self == .bool || (self.rawValue >= Type.int.rawValue && self.rawValue <= Type.string.rawValue)
    }
    
    var isTypedVector : Bool {
        return self == .vector_bool || (self.rawValue >= Type.vector_int.rawValue && self.rawValue <= Type.vector_string.rawValue)
    }
    
    func toTypedVector(length : UInt8 = 0) throws -> Type {
        switch length {
        case 0:
            return Type(rawValue: self.rawValue - Type.int.rawValue + Type.vector_int.rawValue) ?? Type.null
        case 2:
            return Type(rawValue: self.rawValue - Type.int.rawValue + Type.vector_int2.rawValue) ?? Type.null
        case 3:
            return Type(rawValue: self.rawValue - Type.int.rawValue + Type.vector_int3.rawValue) ?? Type.null
        case 4:
            return Type(rawValue: self.rawValue - Type.int.rawValue + Type.vector_int4.rawValue) ?? Type.null
        default:
            throw FlexBufferEncodeError(message: "Typed vector should be of length 0, 2, 3, 4. This one is of lenght \(length)")
        }
    }
    
    var typedVectorElementType : Type {
        return Type(rawValue: self.rawValue - Type.vector_int.rawValue + Type.int.rawValue) ?? Type.null
    }
    
    var isFixedTypedVector : Bool {
        return self.rawValue >= Type.vector_int2.rawValue && self.rawValue <= Type.vector_float4.rawValue
    }
    
    var fixedTypedVectorElementType : (type : Type, length : UInt8) {
        let fixedType = self.rawValue - Type.vector_int2.rawValue
        let length = fixedType / 3 + 2
        let type = Type(rawValue: fixedType % 3 + Type.int.rawValue) ?? Type.null
        return (type: type, length: length)
    }
}

private let widths = [UInt8(1), UInt8(2), UInt8(4), UInt8(8)]

private func packedType(width : BitWidth, type : Type) -> UInt8 {
    return width.rawValue | (type.rawValue << 2)
}

private func nullPackedType() -> UInt8 {
    return packedType(width: .width8, type: .null)
}

private func paddingSize(bufSize : Int, scalarSize : UInt8) -> Int {
    return ((~bufSize) + 1) & (Int(scalarSize) - 1)
}

public struct BuilderOptions : OptionSet {
    public let rawValue: UInt8
    public static let shareKeys = BuilderOptions(rawValue:1 << 0)
    public static let shareStrings = BuilderOptions(rawValue:1 << 1)
    public static let shareKeyVectors = BuilderOptions(rawValue:1 << 2)
    public static let shareKeysAndStrings : BuilderOptions = [.shareKeys, .shareStrings]
    public static let shareAll : BuilderOptions = [.shareKeys, .shareStrings, .shareKeyVectors]
    
    public init(rawValue : UInt8) {
        self.rawValue = rawValue
    }
}

struct FlexBufferEncodeError: Error {
    let message : String
}

// MARK: - JSON Parser
extension FlexBuffer {
    public static func dataFrom(jsonData data : Data, initialSize : Int = 2048, options : BuilderOptions = [], forceNumberParsing: Bool = false) throws -> FlxbData {
        var stack = [Int]()
        var tokenPointerCapacity = 32
        var tokenPointerStart = UnsafeMutablePointer<UInt8>.allocate(capacity: tokenPointerCapacity)
        var tokenPointerCurrent = 0
        
        func addToToken(value : UInt8, tokenPointerCurrent: Int, tokenPointerStart: inout UnsafeMutablePointer<UInt8>) -> Int {
            tokenPointerStart.advanced(by: tokenPointerCurrent).initialize(to: value)
            
            let newTokenPointerCount = tokenPointerCurrent + 1
            if tokenPointerCapacity <= newTokenPointerCount {
                tokenPointerCapacity = tokenPointerCapacity << 1
                let prevBuffer = tokenPointerStart
                tokenPointerStart = UnsafeMutablePointer<UInt8>.allocate(capacity: tokenPointerCapacity)
                tokenPointerStart.initialize(from: prevBuffer, count: newTokenPointerCount)
                prevBuffer.deallocate()
            }
            return newTokenPointerCount
        }
        
        var tokenNamePointerCapacity = 32
        var tokenNamePointerStart = UnsafeMutablePointer<UInt8>.allocate(capacity: tokenPointerCapacity)
        var tokenNamePointerCurrent = 0
        
        var keyIsPresent = false
        var quoteMode = false
        var tokenIsQuoted = false
        let flx = FlexBuffer(initialSize : initialSize, options : options)
        
        func addNumber(_ tokenPointerStart: UnsafeMutablePointer<UInt8>, _ tokenPointerCurrent: Int) throws -> Bool {
            // if a number starts with `0` next character has to be `.` becasaue of JSON specification
            if tokenPointerCurrent >= 2 && tokenPointerStart.advanced(by: 0).pointee == 48 && tokenPointerStart.advanced(by: 1).pointee != 46 {
                return false
            }
            var negative = false
            var expo = false
            var expoNegative = false
            var integer = true
            var intNumber = 0
            var floatNumber = 0
            var expNumber = 0
            var divider = 1
            for i in 0..<tokenPointerCurrent {
                let n1 = tokenPointerStart.advanced(by: i).pointee
                switch n1 {
                case 48...57:
                    let n = Int(tokenPointerStart.advanced(by: i).pointee) - 48
                    if expo {
                        expNumber = expNumber * 10 + n
                    } else if integer {
                        intNumber = intNumber * 10 + n
                    } else {
                        floatNumber = floatNumber * 10 + n
                        divider *= 10
                    }
                case 45: // -
                    if i == 0 {
                        negative = true
                    } else {
                        if expo && !expoNegative {
                            expoNegative = true
                        } else {
                            return false
                        }
                    }
                case 43: // +
                    guard i > 0 else {
                        return false
                    }
                    let prev = tokenPointerStart.advanced(by: i-1).pointee
                    guard prev == 101 || prev == 69 else {
                        return false
                    }
                case 46: // .
                    if i > 0 && integer {
                        integer = false
                    } else {
                        return false
                    }
                case 101, 69: // e, E
                    if !expo {
                        expo = true
                    } else {
                        return false
                    }
                default:
                    return false
                }
            }
            var mult = negative ? -1.0 : 1.0
            if expo {
                mult = mult * pow(Double(10),Double(expoNegative ? -expNumber : expNumber))
            }
            if integer && !expoNegative {
                let n = intNumber * Int(mult)
                if n > Int(INT16_MAX) || n < Int(INT16_MIN) {
                    flx.pushIndirect(n)
                } else {
                    flx.int(n)
                }
            } else {
                let d = (Double(intNumber) + (Double(floatNumber) / Double(divider))) * Double(mult)
                try flx.pushIndirect(d)
            }
            return true
        }
        
        func addValue(_ tokenPointerStart: UnsafeMutablePointer<UInt8>, _ tokenPointerCurrent: Int, _ tokenNamePointerStart : UnsafeMutablePointer<UInt8>, tokenNamePointerCurrent : Int) throws {
            
            if keyIsPresent {
                flx.key(tokenNamePointerStart, tokenNamePointerCurrent)
            }
            
            if tokenPointerCurrent == 4 && tokenPointerStart.pointee == 110 && tokenPointerStart.advanced(by: 1).pointee == 117 && tokenPointerStart.advanced(by: 2).pointee == 108 && tokenPointerStart.advanced(by: 3).pointee == 108 {
                flx.addNull()
                return
            }
            
            if tokenPointerCurrent == 4 && tokenPointerStart.pointee == 116 && tokenPointerStart.advanced(by: 1).pointee == 114 && tokenPointerStart.advanced(by: 2).pointee == 117 && tokenPointerStart.advanced(by: 3).pointee == 101 {
                flx.bool(true)
                return
            }
            
            if tokenPointerCurrent == 5 && tokenPointerStart.pointee == 102 && tokenPointerStart.advanced(by: 1).pointee == 97 && tokenPointerStart.advanced(by: 2).pointee == 108 && tokenPointerStart.advanced(by: 3).pointee == 115 && tokenPointerStart.advanced(by: 4).pointee == 101 {
                flx.bool(false)
                return
            }
            
            if tokenIsQuoted == false || forceNumberParsing {
                if try addNumber(tokenPointerStart, tokenPointerCurrent) {
                    return
                }
            }
            
            flx.string(tokenPointerStart, tokenPointerCurrent)
            
        }
        
        func convertHexToNumber(_ c : UInt8) -> Int {
            if c >= 48 && c<=57 {
                return Int(c) - 48
            } else {
                return  Int(c) - 97 + 10
            }
        }

        try data.withUnsafeBytes{ (rawBytes) -> Void in
            let bytes = rawBytes.baseAddress!.assumingMemoryBound(to: UInt8.self)
            var i : Int = 0
            while i<data.count {
                let c = bytes.advanced(by: i)
                switch c.pointee {
                case 92: // \
                    if quoteMode {
                        if c.advanced(by: 1).pointee == 92 // handling \\ case
                        || c.advanced(by: 1).pointee == 47 // handling \/ case 
                        || c.advanced(by: 1).pointee == 34 { // handling \" case
                            i += 1
                            tokenPointerCurrent = addToToken(value: c.advanced(by: 1).pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                        } else if c.advanced(by: 1).pointee == 117 && data.count > i + 5 { // handling \u case + four-hex-digits
                            let n = convertHexToNumber(c.advanced(by: 5).pointee) +
                                    (convertHexToNumber(c.advanced(by: 4).pointee) * 16) +
                                    (convertHexToNumber(c.advanced(by: 3).pointee) * 256) +
                                    (convertHexToNumber(c.advanced(by: 2).pointee) * 4096)
                            if let unicodeScalar = UnicodeScalar(n) {
                                UTF8.encode(unicodeScalar, into: { (c) in
                                    tokenPointerCurrent = addToToken(value: c, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                                })
                                i += 5
                            } else {
                                fallthrough
                            }
                        } else {
                            fallthrough
                        }
                    } else {
                        fallthrough
                    }
                case 123: //{
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    } else {
                        if keyIsPresent {
                            flx.key(tokenNamePointerStart, tokenNamePointerCurrent)
                        }
                        keyIsPresent = false
                        stack.append(flx.startMap())
                        tokenNamePointerCurrent = 0
                        tokenPointerCurrent = 0
                    }
                case 91: //[
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    } else {
                        if keyIsPresent {
                            flx.key(tokenNamePointerStart, tokenNamePointerCurrent)
                        }
                        keyIsPresent = false
                        stack.append(flx.startVector())
                        tokenNamePointerCurrent = 0
                        tokenPointerCurrent = 0
                    }
                case 125: //}
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    } else {
                        if tokenPointerCurrent > 0 {
                            try addValue(tokenPointerStart, tokenPointerCurrent, tokenNamePointerStart, tokenNamePointerCurrent: tokenNamePointerCurrent)
                            tokenIsQuoted = false
                        }
                        try flx.endMap(start: stack.removeLast())
                        tokenNamePointerCurrent = 0
                        tokenPointerCurrent = 0
                        keyIsPresent = false
                    }
                case 93: //]
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    } else {
                        
                        if tokenPointerCurrent > 0 {
                            try addValue(tokenPointerStart, tokenPointerCurrent, tokenNamePointerStart, tokenNamePointerCurrent: tokenNamePointerCurrent)
                            tokenIsQuoted = false
                        }
                        _ = try flx.endVector(start: stack.removeLast(), typed: false, fixed: false)
                        tokenNamePointerCurrent = 0
                        tokenPointerCurrent = 0
                        keyIsPresent = false
                    }
                case 58: //:
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    } else {
                        if stack.count == 0 {
                            return
                        }
                        if tokenNamePointerCapacity < tokenPointerCurrent {
                            tokenNamePointerStart.deallocate()
                            tokenNamePointerStart = UnsafeMutablePointer<UInt8>.allocate(capacity: tokenPointerCurrent)
                            tokenNamePointerCapacity = tokenPointerCurrent
                        }
                        tokenNamePointerStart.moveUpdate(from: tokenPointerStart, count: tokenPointerCurrent)
                        tokenNamePointerCurrent = tokenPointerCurrent
                        tokenPointerCurrent = 0
                        keyIsPresent = true
                        tokenIsQuoted = false
                    }
                case 34://"
                        quoteMode = quoteMode != true
                        tokenIsQuoted = tokenIsQuoted || quoteMode
                case 44://,
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    } else {
                        if tokenPointerCurrent > 0 {
                            try addValue(tokenPointerStart, tokenPointerCurrent, tokenNamePointerStart, tokenNamePointerCurrent: tokenNamePointerCurrent)
                        }
                        tokenIsQuoted = false
                        tokenNamePointerCurrent = 0
                        tokenPointerCurrent = 0
                        keyIsPresent = false
                    }
                case 10, 13: // \n, \r
                    break
                case 9, 32: // \t space
                    if quoteMode {
                        tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                    }
                default:
                    tokenPointerCurrent = addToToken(value: c.pointee, tokenPointerCurrent: tokenPointerCurrent, tokenPointerStart: &tokenPointerStart)
                }
                i += 1
            }
        }
        tokenPointerStart.deallocate()
        tokenNamePointerStart.deallocate()
        let flxbData = try flx.finish()
        return FlxbData(data: flxbData)
    }
}
