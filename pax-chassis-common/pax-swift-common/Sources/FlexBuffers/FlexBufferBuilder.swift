//
//  FlexBufferBuilder.swift
//  FlexBuffers
//
//  Created by Maxim Zaks on 19.08.17.
//  Copyright © 2017 Maxim Zaks. All rights reserved.
//

import Foundation


public enum FlexBufferBuilder {
    public static func encodeMap(initialSize : Int = 2048, options : BuilderOptions = [], _ block: (FlexBufferMapBuilder) throws -> () ) throws -> FlxbData {
        let builder = FlexBufferMapBuilder(flxb: FlexBuffer(initialSize: initialSize, options: options))
        try block(builder)
        try builder.end()
        let data = try builder.flxb.finish()
        return FlxbData(data: data)
    }
    public static func encodeVector(initialSize : Int = 2048, options : BuilderOptions = [], _ block: (FlexBufferVectorBuilder) throws -> () ) throws -> FlxbData {
        let builder = FlexBufferVectorBuilder(flxb: FlexBuffer(initialSize: initialSize, options: options))
        try block(builder)
        try builder.end()
        let data = try builder.flxb.finish()
        return FlxbData(data: data)
    }
    public static func fromJSON(_ data: Data) throws -> FlxbData {
        return try FlexBuffer.dataFrom(jsonData: data)
    }
    public static func fromJSON(_ string: String) throws -> FlxbData {
        guard let data = string.data(using: .utf8) else {
            throw FlexBufferBuilderErrors(errors: [])
        }
        return try FlexBuffer.dataFrom(jsonData: data)
    }
    public static func encode<V: FlxbValue>(_ value: V) throws -> FlxbData {
        return try FlexBuffer.encode(value)
    }
}


public struct FlexBufferBuilderErrors: Error {
    let errors : [Error]
}

public final class FlexBufferMapBuilder {
    fileprivate let flxb: FlexBuffer
    var start : Int
    var errors : [Error] = []
    
    init(flxb: FlexBuffer) {
        self.flxb = flxb
        start = flxb.startMap()
    }
    
    public func add(_ key: StaticString, _ value: Int) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: Int64) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: (Int, Int)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: (Int, Int, Int)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: (Int, Int, Int, Int)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: [Int]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: [Int64]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addWithStringKey(_ key: String, _ value: Int) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: Int64) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: (Int, Int)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: (Int, Int, Int)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: (Int, Int, Int, Int)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: [Int]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: [Int64]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ key: StaticString, _ value: UInt) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: UInt64) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: (UInt, UInt)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: (UInt, UInt, UInt)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: (UInt, UInt, UInt, UInt)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: [UInt]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: [UInt64]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addWithStringKey(_ key: String, _ value: UInt) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: UInt64) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: (UInt, UInt)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: (UInt, UInt, UInt)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: (UInt, UInt, UInt, UInt)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: [UInt]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: [UInt64]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ key: StaticString, _ value: Double) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: (Double, Double)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: (Double, Double, Double)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: (Double, Double, Double, Double)) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: [Double]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addWithStringKey(_ key: String, _ value: Double) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: (Double, Double)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: (Double, Double, Double)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: (Double, Double, Double, Double)) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: [Double]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ key: StaticString, _ value: Bool) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: [Bool]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: Bool) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: [Bool]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addString(_ key: StaticString, _ value: String) {
        flxb.add(key: key, stringValue: value)
    }
    public func addString(_ key: StaticString, _ value: [String]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ key: StaticString, _ value: StaticString) {
        flxb.add(key: key, value: value)
    }
    public func add(_ key: StaticString, _ value: [StaticString]) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addStringWithStringKey(_ key: String, _ value: String) {
        flxb.add(keyString: key, stringValue: value)
    }
    public func addStringWithStringKey(_ key: String, _ value: [String]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func addWithStringKey(_ key: String, _ value: StaticString) {
        flxb.add(keyString: key, value: value)
    }
    public func addWithStringKey(_ key: String, _ value: [StaticString]) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ key: StaticString, _ value: Data) {
        do {
            try flxb.add(key: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addWithStringKey(_ key: String, _ value: Data) {
        do {
            try flxb.add(keyString: key, value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addNull(_ key: StaticString) {
        flxb.addNull(key: key)
    }
    
    public func addNullWithStringKey(_ key: String) {
        flxb.addNull(keyString: key)
    }
    
    public func indirectAdd(_ key: StaticString, _ value: Int) {
        flxb.add(key: key, indirectValue: value)
    }
    public func indirectAdd(_ key: StaticString, _ value: Int64) {
        flxb.add(key: key, indirectValue: value)
    }
    public func indirectAdd(_ key: StaticString, _ value: UInt) {
        flxb.add(key: key, indirectValue: value)
    }
    public func indirectAdd(_ key: StaticString, _ value: UInt64) {
        flxb.add(key: key, indirectValue: value)
    }
    public func indirectAdd(_ key: StaticString, _ value: Double) {
        do {
            try flxb.add(key: key, indirectValue: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func indirectAddWithStringKey(_ key: String, _ value: Int) {
        flxb.add(keyString: key, indirectValue: value)
    }
    public func indirectAddWithStringKey(_ key: String, _ value: Int64) {
        flxb.add(keyString: key, indirectValue: value)
    }
    public func indirectAddWithStringKey(_ key: String, _ value: UInt) {
        flxb.add(keyString: key, indirectValue: value)
    }
    public func indirectAddWithStringKey(_ key: String, _ value: UInt64) {
        flxb.add(keyString: key, indirectValue: value)
    }
    public func indirectAddWithStringKey(_ key: String, _ value: Double) {
        do {
            try flxb.add(keyString: key, indirectValue: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    var _vectorBuilder: FlexBufferVectorBuilder?
    var vectorpBuilder: FlexBufferVectorBuilder {
        if let _builder = _vectorBuilder {
            _builder.start = flxb.startMap()
            _builder.errors.removeAll()
            return _builder
        } else {
            let _builder = FlexBufferVectorBuilder(flxb: self.flxb)
            _vectorBuilder = _builder
            return _builder
        }
    }
    public func addVector(_ key: StaticString, _ block: (FlexBufferVectorBuilder) throws -> ()) {
        self.flxb.key(key)
        let builder = FlexBufferVectorBuilder(flxb: self.flxb)
        do {
            try block(builder)
            try builder.end()
        } catch let error {
            errors.append(error)
        }
    }
    public func addVectorWithStringKey(_ key: String, _ block: (FlexBufferVectorBuilder) throws -> ()) {
        self.flxb.key(key)
        let builder = FlexBufferVectorBuilder(flxb: self.flxb)
        do {
            try block(builder)
            try builder.end()
        } catch let error {
            errors.append(error)
        }
    }
    
    var _mapBuilder: FlexBufferMapBuilder?
    var mapBuilder: FlexBufferMapBuilder {
        if let _builder = _mapBuilder {
            _builder.start = flxb.startMap()
            _builder.errors.removeAll()
            return _builder
        } else {
            let _builder = FlexBufferMapBuilder(flxb: self.flxb)
            _mapBuilder = _builder
            return _builder
        }
    }
    
    public func addMap(_ key: StaticString, _ block: (FlexBufferMapBuilder) throws -> ()) {
        self.flxb.key(key)
        let builder = mapBuilder
        do {
            try block(builder)
            try builder.end()
        } catch let error {
            errors.append(error)
        }
    }
    public func addMapWithStringKey(_ key: String, _ block: (FlexBufferMapBuilder) throws -> ()) {
        self.flxb.key(key)
        let builder = mapBuilder
        do {
            try block(builder)
            try builder.end()
        } catch let error {
            errors.append(error)
        }
    }
    
    fileprivate func end() throws {
        guard errors.isEmpty else {
            throw FlexBufferBuilderErrors(errors: errors)
        }
        try flxb.endMap(start: start)
    }
}

public final class FlexBufferVectorBuilder {
    fileprivate let flxb: FlexBuffer
    var start : Int
    var errors : [Error] = []
    
    init(flxb: FlexBuffer) {
        self.flxb = flxb
        start = flxb.startVector()
    }
    
    public func add(_ value: Int) {
        flxb.add(value: value)
    }
    public func add(_ value: Int64) {
        flxb.add(value: value)
    }
    public func add(_ value: [Int]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: [Int64]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (Int, Int)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (Int, Int, Int)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (Int, Int, Int, Int)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ value: UInt) {
        flxb.add(value: value)
    }
    public func add(_ value: UInt64) {
        flxb.add(value: value)
    }
    public func add(_ value: [UInt]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: [UInt64]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (UInt, UInt)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (UInt, UInt, UInt)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (UInt, UInt, UInt, UInt)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ value: Double) {
        flxb.add(value: value)
    }
    public func add(_ value: [Double]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (Double, Double)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (Double, Double, Double)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    public func add(_ value: (Double, Double, Double, Double)) {
        do {
            try flxb.add(value: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ value: Bool) {
        flxb.add(value: value)
    }
    public func add(_ value: [Bool]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func add(_ value: StaticString) {
        flxb.add(value: value)
    }
    public func add(_ value: [StaticString]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addString(_ value: String) {
        flxb.add(stringValue: value)
    }
    public func addString(_ value: [String]) {
        do {
            try flxb.add(array: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    public func addNull() {
        flxb.addNull()
    }
    
    public func add(_ value: Data) {
        flxb.add(value: value)
    }
    
    var _vectorBuilder: FlexBufferVectorBuilder?
    var vectorpBuilder: FlexBufferVectorBuilder {
        if let _builder = _vectorBuilder {
            _builder.start = flxb.startMap()
            _builder.errors.removeAll()
            return _builder
        } else {
            let _builder = FlexBufferVectorBuilder(flxb: self.flxb)
            _vectorBuilder = _builder
            return _builder
        }
    }
    
    public func addVector(_ block: (FlexBufferVectorBuilder) throws -> ()) {
        let builder = vectorpBuilder
        do {
            try block(builder)
            try builder.end()
        } catch let error {
            errors.append(error)
        }
    }
    
    var _mapBuilder: FlexBufferMapBuilder?
    var mapBuilder: FlexBufferMapBuilder {
        if let _builder = _mapBuilder {
            _builder.start = flxb.startMap()
            _builder.errors.removeAll()
            return _builder
        } else {
            let _builder = FlexBufferMapBuilder(flxb: self.flxb)
            _mapBuilder = _builder
            return _builder
        }
    }
    
    public func addMap(_ block: (FlexBufferMapBuilder) throws -> ()) {
        let builder = mapBuilder
        do {
            try block(builder)
            try builder.end()
        } catch let error {
            errors.append(error)
        }
    }
    
    public func indirectAdd(_ value: Int) {
        flxb.add(indirectValue: value)
    }
    public func indirectAdd(_ value: Int64) {
        flxb.add(indirectValue: value)
    }
    public func indirectAdd(_ value: UInt) {
        flxb.add(indirectValue: value)
    }
    public func indirectAdd(_ value: UInt64) {
        flxb.add(indirectValue: value)
    }
    public func indirectAdd(_ value: Double) {
        do {
            try flxb.add(indirectValue: value)
        } catch let error {
            errors.append(error)
        }
    }
    
    fileprivate func end() throws {
        guard errors.isEmpty else {
            throw FlexBufferBuilderErrors(errors: errors)
        }
        _ = try flxb.endVector(start: start, typed: false, fixed: false)
    }
}
