//
//  Messages.swift
//  interface
//
//  Created by Zachary Brown on 5/7/22.
//

import Foundation
import SwiftUI
import FlexBuffers
#if os(iOS) || os(tvOS) || os(watchOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

public typealias PaxNodeId = UInt32

private func decodeId(_ fb: FlxbReference) -> UInt32? {
    if let id = fb["id"]?.asUInt64 {
        return UInt32(id)
    }
    if let id = fb.asUInt64 {
        return UInt32(id)
    }
    if let idChain = fb["id_chain"]?.asVector {
        let values = idChain.makeIterator().compactMap { $0.asUInt64 }
        if values.count == 1, let id = values.first {
            return UInt32(id)
        }
    }
    return nil
}

private func decodeIdChain(_ fb: FlxbReference) -> [UInt64] {
    if let idChain = fb["id_chain"]?.asVector {
        return idChain.makeIterator().compactMap { $0.asUInt64 }
    }
    if let id = decodeId(fb) {
        return [UInt64(id)]
    }
    return []
}

private func decodeClippingIds(_ fb: FlxbReference) -> [[UInt64]] {
    guard let clippingIds = fb["clipping_ids"]?.asVector else {
        return []
    }
    return clippingIds.makeIterator().map { clippingId in
        clippingId.asVector?.makeIterator().compactMap { $0.asUInt64 } ?? []
    }
}

public extension Notification.Name {
    static let paxFontRegistered = Notification.Name("PaxFontRegistered")
}

public final class NativeInterruptDispatcher {
    public static let shared = NativeInterruptDispatcher()

    public var sendData: ((Data) -> Void)?

    private init() {}

    public func send(_ data: Data) {
        sendData?(data)
    }
}

public struct PhotoPickerSelectedAsset {
    public let tempId: String
    public let fileName: String?
    public let mimeType: String
    public let byteSize: UInt64
    public let width: UInt32?
    public let height: UInt32?
    public let sourceKind: String
    public let handle: String?

    public init(
        tempId: String,
        fileName: String?,
        mimeType: String,
        byteSize: UInt64,
        width: UInt32?,
        height: UInt32?,
        sourceKind: String,
        handle: String?
    ) {
        self.tempId = tempId
        self.fileName = fileName
        self.mimeType = mimeType
        self.byteSize = byteSize
        self.width = width
        self.height = height
        self.sourceKind = sourceKind
        self.handle = handle
    }
}

struct RouteLocationMessage: Equatable {
    let pathSegments: [String]
    let query: [String: [String]]
    let fragment: String?

    init(pathSegments: [String], query: [String: [String]], fragment: String?) {
        self.pathSegments = pathSegments
        self.query = query
        self.fragment = fragment
    }
}

final class PaxVirtualRouteCoordinator {
    static let shared = PaxVirtualRouteCoordinator()

    private static let virtualScheme = "pax"
    private static let virtualHost = "app"

    private var currentLocation = RouteLocationMessage(
        pathSegments: [],
        query: [:],
        fragment: nil
    )

    init() {}

    func resolve(destination: String) -> RouteLocationMessage? {
        guard let destinationComponents = URLComponents(string: destination),
              destinationComponents.scheme == nil,
              destinationComponents.host == nil,
              let baseURL = baseURL(),
              let resolvedURL = URL(string: destination, relativeTo: baseURL)?.absoluteURL,
              let resolvedComponents = URLComponents(url: resolvedURL, resolvingAgainstBaseURL: false),
              resolvedComponents.scheme == Self.virtualScheme,
              resolvedComponents.host == Self.virtualHost else {
            return nil
        }

        return RouteLocationMessage(
            pathSegments: Self.pathSegments(from: resolvedComponents.path),
            query: Self.query(from: resolvedComponents.queryItems),
            fragment: resolvedComponents.fragment
        )
    }

    @discardableResult
    func navigate(to destination: String) -> Bool {
        guard let location = resolve(destination: destination) else {
            return false
        }
        currentLocation = location
        dispatchRouteChange(location)
        return true
    }

    private func baseURL() -> URL? {
        var components = URLComponents()
        components.scheme = Self.virtualScheme
        components.host = Self.virtualHost
        components.path = "/" + currentLocation.pathSegments.joined(separator: "/")
        components.queryItems = currentLocation.query.flatMap { key, values in
            values.map { URLQueryItem(name: key, value: $0) }
        }
        components.fragment = currentLocation.fragment
        return components.url
    }

    private static func pathSegments(from path: String) -> [String] {
        path.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
    }

    private static func query(from queryItems: [URLQueryItem]?) -> [String: [String]] {
        var query: [String: [String]] = [:]
        for item in queryItems ?? [] {
            query[item.name, default: []].append(item.value ?? "")
        }
        return query
    }
}

public struct TouchInterruptMessage {
    public let x: Double
    public let y: Double
    public let identifier: Int64
    public let deltaX: Double
    public let deltaY: Double

    public init(x: Double, y: Double, identifier: Int64, deltaX: Double, deltaY: Double) {
        self.x = x
        self.y = y
        self.identifier = identifier
        self.deltaX = deltaX
        self.deltaY = deltaY
    }
}

private final class PaxWebFontLoader {
    static let shared = PaxWebFontLoader()

    private let stateQueue = DispatchQueue(label: "dev.pax.font-loader")
    private var inFlightSources: Set<String> = []
    private var loadedSources: Set<String> = []

    private init() {}

    func ensureLoaded(font: PaxFont.WebFont) {
        let sourceKey = font.url.absoluteString
        let shouldStart = stateQueue.sync { () -> Bool in
            if loadedSources.contains(sourceKey) || inFlightSources.contains(sourceKey) {
                return false
            }
            inFlightSources.insert(sourceKey)
            return true
        }
        guard shouldStart else {
            return
        }

        load(font: font, sourceKey: sourceKey)
    }

    private func finish(sourceKey: String, didLoad: Bool) {
        stateQueue.async {
            self.inFlightSources.remove(sourceKey)
            if didLoad {
                self.loadedSources.insert(sourceKey)
            }
        }
    }

    private func load(font: PaxFont.WebFont, sourceKey: String) {
        if font.url.absoluteString.contains("fonts.googleapis.com/css") {
            URLSession.shared.dataTask(with: font.url) { data, _, error in
                guard let data, error == nil, let css = String(data: data, encoding: .utf8) else {
                    self.finish(sourceKey: sourceKey, didLoad: false)
                    return
                }
                let assetURLs = self.parseCSSFontURLs(from: css, baseURL: font.url)
                guard !assetURLs.isEmpty else {
                    self.finish(sourceKey: sourceKey, didLoad: false)
                    return
                }
                self.loadAssetURLs(assetURLs, expectedFamily: font.family) { didLoad in
                    self.finish(sourceKey: sourceKey, didLoad: didLoad)
                }
            }.resume()
        } else {
            loadAssetURLs([font.url], expectedFamily: font.family) { didLoad in
                self.finish(sourceKey: sourceKey, didLoad: didLoad)
            }
        }
    }

    private func loadAssetURLs(_ urls: [URL], expectedFamily: String, completion: @escaping (Bool) -> Void) {
        let group = DispatchGroup()
        let resultQueue = DispatchQueue(label: "dev.pax.font-loader.results")
        var didLoadAny = false

        for assetURL in urls {
            group.enter()
            URLSession.shared.dataTask(with: assetURL) { data, _, error in
                defer { group.leave() }
                guard let data, error == nil else {
                    return
                }
                if self.registerFontData(data, sourceURL: assetURL, expectedFamily: expectedFamily) {
                    resultQueue.sync {
                        didLoadAny = true
                    }
                }
            }.resume()
        }

        group.notify(queue: .main) {
            completion(didLoadAny)
        }
    }

    private func registerFontData(_ data: Data, sourceURL: URL, expectedFamily: String) -> Bool {
        let fileManager = FileManager.default
        let temporaryURL = fileManager.temporaryDirectory
            .appendingPathComponent("pax-font-\(UUID().uuidString)")
            .appendingPathExtension(sourceURL.pathExtension.isEmpty ? "font" : sourceURL.pathExtension)

        do {
            try data.write(to: temporaryURL, options: .atomic)
        } catch {
            return false
        }

        defer {
            try? fileManager.removeItem(at: temporaryURL)
        }

        var errorRef: Unmanaged<CFError>?
        let registered = CTFontManagerRegisterFontsForURL(temporaryURL as CFURL, .process, &errorRef)
        if !registered {
            return false
        }

        var didMarkFamily = false
        var didRegisterNewFont = false
        if let descriptors = CTFontManagerCreateFontDescriptorsFromURL(temporaryURL as CFURL) as? [CTFontDescriptor] {
            for descriptor in descriptors {
                if let fontFamily = CTFontDescriptorCopyAttribute(descriptor, kCTFontFamilyNameAttribute) as? String {
                    didRegisterNewFont = PaxFont.markFontRegistered(fontFamily: fontFamily) || didRegisterNewFont
                    didMarkFamily = didMarkFamily || (fontFamily == expectedFamily)
                }
                if let postscriptName = CTFontDescriptorCopyAttribute(descriptor, kCTFontNameAttribute) as? String {
                    didRegisterNewFont = PaxFont.markFontRegistered(fontFamily: postscriptName) || didRegisterNewFont
                }
            }
        }

        if !didMarkFamily {
            didRegisterNewFont = PaxFont.markFontRegistered(fontFamily: expectedFamily) || didRegisterNewFont
        }

        if didRegisterNewFont {
            NotificationCenter.default.post(name: .paxFontRegistered, object: expectedFamily)
        }
        return true
    }

    private func parseCSSFontURLs(from css: String, baseURL: URL) -> [URL] {
        let pattern = #"url\((['"]?)([^'")]+)\1\)"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else {
            return []
        }

        let nsRange = NSRange(css.startIndex..<css.endIndex, in: css)
        var urls: [URL] = []
        var seen: Set<String> = []
        regex.enumerateMatches(in: css, options: [], range: nsRange) { match, _, _ in
            guard
                let match,
                match.numberOfRanges >= 3,
                let valueRange = Range(match.range(at: 2), in: css)
            else {
                return
            }

            let rawValue = String(css[valueRange])
            let resolvedURL = URL(string: rawValue, relativeTo: baseURL)?.absoluteURL
            guard let resolvedURL else {
                return
            }
            let key = resolvedURL.absoluteString
            guard !seen.contains(key) else {
                return
            }
            seen.insert(key)
            urls.append(resolvedURL)
        }
        return urls
    }
}

private func dispatchNativeInterrupt(_ build: (FlexBufferMapBuilder) throws -> Void) {
    let buffer = try! FlexBufferBuilder.encodeMap(build)
    NativeInterruptDispatcher.shared.send(buffer.data)
}

private func dispatchTouchInterrupt(type: String, touches: [TouchInterruptMessage]) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey(type) { messageBuilder in
            messageBuilder.addVectorWithStringKey("touches") { vectorBuilder in
                for touch in touches {
                    vectorBuilder.addMap { touchBuilder in
                        touchBuilder.addWithStringKey("x", touch.x)
                        touchBuilder.addWithStringKey("y", touch.y)
                        touchBuilder.addWithStringKey("identifier", Int(touch.identifier))
                        touchBuilder.addWithStringKey("delta_x", touch.deltaX)
                        touchBuilder.addWithStringKey("delta_y", touch.deltaY)
                    }
                }
            }
        }
    }
}

public func dispatchTap(x: Double, y: Double) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("Tap") { messageBuilder in
            messageBuilder.addWithStringKey("x", x)
            messageBuilder.addWithStringKey("y", y)
        }
    }
}

func dispatchRouteChange(_ location: RouteLocationMessage) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("RouteChange") { messageBuilder in
            messageBuilder.addVectorWithStringKey("path_segments") { segmentsBuilder in
                for segment in location.pathSegments {
                    segmentsBuilder.addString(segment)
                }
            }
            messageBuilder.addMapWithStringKey("query") { queryBuilder in
                for (key, values) in location.query {
                    queryBuilder.addVectorWithStringKey(key) { valuesBuilder in
                        for value in values {
                            valuesBuilder.addString(value)
                        }
                    }
                }
            }
            if let fragment = location.fragment {
                messageBuilder.addStringWithStringKey("fragment", fragment)
            } else {
                messageBuilder.addNullWithStringKey("fragment")
            }
        }
    }
}

@discardableResult
public func dispatchVirtualRouteNavigation(to destination: String) -> Bool {
    PaxVirtualRouteCoordinator.shared.navigate(to: destination)
}

public func dispatchGyro(x: Double, y: Double, z: Double) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("Gyro") { messageBuilder in
            messageBuilder.addWithStringKey("x", x)
            messageBuilder.addWithStringKey("y", y)
            messageBuilder.addWithStringKey("z", z)
        }
    }
}

public func dispatchAccel(x: Double, y: Double, z: Double) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("Accel") { messageBuilder in
            messageBuilder.addWithStringKey("x", x)
            messageBuilder.addWithStringKey("y", y)
            messageBuilder.addWithStringKey("z", z)
        }
    }
}

public func dispatchChassisResizeRequest(id: PaxNodeId, width: Double, height: Double) {
    dispatchNativeInterrupt { builder in
        builder.addVectorWithStringKey("ChassisResizeRequestCollection") { vectorBuilder in
            vectorBuilder.addMap { requestBuilder in
                requestBuilder.addWithStringKey("id", UInt(id))
                requestBuilder.addWithStringKey("width", width)
                requestBuilder.addWithStringKey("height", height)
            }
        }
    }
}

public func dispatchTouchStart(touches: [TouchInterruptMessage]) {
    dispatchTouchInterrupt(type: "TouchStart", touches: touches)
}

public func dispatchTouchMove(touches: [TouchInterruptMessage]) {
    dispatchTouchInterrupt(type: "TouchMove", touches: touches)
}

public func dispatchTouchEnd(touches: [TouchInterruptMessage]) {
    dispatchTouchInterrupt(type: "TouchEnd", touches: touches)
}

public func dispatchFormButtonClick(id: PaxNodeId) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormButtonClick") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
        }
    }
}

public func dispatchPhotoPicker(
    id: PaxNodeId,
    requestId: UInt64,
    status: String,
    message: String?,
    photos: [PhotoPickerSelectedAsset]
) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("PhotoPicker") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("request_id", UInt(requestId))
            messageBuilder.addStringWithStringKey("status", status)
            if let message {
                messageBuilder.addStringWithStringKey("message", message)
            }
            messageBuilder.addVectorWithStringKey("photos") { vectorBuilder in
                for photo in photos {
                    vectorBuilder.addMap { photoBuilder in
                        photoBuilder.addStringWithStringKey("temp_id", photo.tempId)
                        if let fileName = photo.fileName {
                            photoBuilder.addStringWithStringKey("file_name", fileName)
                        }
                        photoBuilder.addStringWithStringKey("mime_type", photo.mimeType)
                        photoBuilder.addWithStringKey("byte_size", UInt(photo.byteSize))
                        if let width = photo.width {
                            photoBuilder.addWithStringKey("width", UInt(width))
                        }
                        if let height = photo.height {
                            photoBuilder.addWithStringKey("height", UInt(height))
                        }
                        photoBuilder.addStringWithStringKey("source_kind", photo.sourceKind)
                        if let handle = photo.handle {
                            photoBuilder.addStringWithStringKey("handle", handle)
                        }
                    }
                }
            }
        }
    }
}

public func dispatchFormCheckboxToggle(id: PaxNodeId, state: Bool) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormCheckboxToggle") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("state", state)
        }
    }
}

public func dispatchFormDropdownChange(id: PaxNodeId, selectedId: UInt32) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormDropdownChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("selected_id", UInt(selectedId))
        }
    }
}

public func dispatchFormRadioListChange(id: PaxNodeId, selectedId: UInt32) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormRadioListChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("selected_id", UInt(selectedId))
        }
    }
}

public func dispatchFormSliderChange(id: PaxNodeId, value: Double) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormSliderChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("value", value)
        }
    }
}

public func dispatchFormTextboxInput(id: PaxNodeId, text: String) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormTextboxInput") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addStringWithStringKey("text", text)
        }
    }
}

public func dispatchFormTextboxChange(id: PaxNodeId, text: String) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("FormTextboxChange") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addStringWithStringKey("text", text)
        }
    }
}

public func dispatchTextInput(id: PaxNodeId, text: String) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("TextInput") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addStringWithStringKey("text", text)
        }
    }
}

public func dispatchScrollbarChange(
    id: PaxNodeId,
    scrollX: Double,
    scrollY: Double,
    presentationScrollX: Double? = nil,
    presentationScrollY: Double? = nil
) {
    dispatchNativeInterrupt { builder in
        builder.addMapWithStringKey("Scrollbar") { messageBuilder in
            messageBuilder.addWithStringKey("id", UInt(id))
            messageBuilder.addWithStringKey("scroll_x", scrollX)
            messageBuilder.addWithStringKey("scroll_y", scrollY)
            if let presentationScrollX {
                messageBuilder.addWithStringKey("presentation_scroll_x", presentationScrollX)
            }
            if let presentationScrollY {
                messageBuilder.addWithStringKey("presentation_scroll_y", presentationScrollY)
            }
        }
    }
}

private func readNodeId(_ fb: FlxbReference?) -> PaxNodeId? {
    guard let fb, !fb.isNull else {
        return nil
    }
    if let value = fb.asUInt64 {
        return PaxNodeId(truncatingIfNeeded: value)
    }
    if let value = fb.asUInt {
        return PaxNodeId(truncatingIfNeeded: value)
    }
    return nil
}

private func fieldExists(_ fb: FlxbReference, _ key: String) -> Bool {
    fb.get(key: key) != nil
}

private func readFloatArray(_ fb: FlxbReference?) -> [Float]? {
    guard let vector = fb?.asVector else {
        return nil
    }
    return vector.makeIterator().map { item in
        if let value = item.asFloat {
            return value
        }
        if let value = item.asDouble {
            return Float(value)
        }
        return 0.0
    }
}

private func readDoubleArray(_ fb: FlxbReference?) -> [Double]? {
    guard let vector = fb?.asVector else {
        return nil
    }
    return vector.makeIterator().map { item in
        if let value = item.asDouble {
            return value
        }
        if let value = item.asFloat {
            return Double(value)
        }
        if let value = item.asInt {
            return Double(value)
        }
        return 0.0
    }
}

private func readStringArray(_ fb: FlxbReference?) -> [String]? {
    guard let vector = fb?.asVector else {
        return nil
    }
    return vector.makeIterator().compactMap { $0.asString }
}

private func readMaskEntries(_ fb: FlxbReference?) -> [MaskPathPatch]? {
    guard let vector = fb?.asVector else {
        return nil
    }
    return vector.makeIterator().compactMap { entry in
        guard let path = entry["path"]?.asString else {
            return nil
        }
        return MaskPathPatch(
            path: path,
            clips: readStringArray(entry["clips"]) ?? [],
            opacity: readDouble(entry["opacity"]) ?? 1.0
        )
    }
}

private func readDouble(_ fb: FlxbReference?) -> Double? {
    if let value = fb?.asDouble {
        return value
    }
    if let value = fb?.asFloat {
        return Double(value)
    }
    return nil
}

private func readInt(_ fb: FlxbReference?) -> Int? {
    if let value = fb?.asInt {
        return Int(value)
    }
    if let value = fb?.asUInt64 {
        return Int(truncatingIfNeeded: value)
    }
    if let value = fb?.asUInt {
        return Int(truncatingIfNeeded: value)
    }
    return nil
}

/// Agnostic of the type of element, this patch contains only create-time metadata.
public class AnyCreatePatch {
    public var id: PaxNodeId
    public var id_chain: [UInt64]
    /// Used for clipping -- each `[UInt64]` is an `id_chain` for an associated clipping mask (`Frame`)
    public var clipping_ids: [[UInt64]]
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    
    public init(fb:FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? decodeId(fb) ?? 0
        self.id_chain = decodeIdChain(fb)
        self.clipping_ids = decodeClippingIds(fb)
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.renderLayerId = UInt32(truncatingIfNeeded: fb["render_layer_id"]?.asUInt64 ?? 0)
    }
}


public class AnyDeletePatch {
    public var id: PaxNodeId
    public var id_chain: [UInt64]
    
    public init(fb:FlxbReference) {
        self.id = readNodeId(fb) ?? decodeId(fb) ?? 0
        self.id_chain = decodeIdChain(fb)
    }
}

public class TextStyle {
    public var font: PaxFont
    public var fill: Color
    public var alignmentMultiline: TextAlignment
    public var alignment: Alignment
    public var font_size: CGFloat
    public var underline: Bool
    
    public init(font: PaxFont, fill: Color, alignmentMultiline: TextAlignment, alignment: Alignment, font_size: CGFloat, underline: Bool) {
        self.font = font
        self.fill = fill
        self.alignmentMultiline = alignmentMultiline
        self.alignment = alignment
        self.font_size = font_size
        self.underline = underline
    }
    
    public func applyPatch(from patch: TextStyleMessage) {
        
        self.font.applyPatch(fb: patch.font)

        if patch.fill != nil {
            self.fill = patch.fill!
        }
        
        if patch.align_multiline != nil {
            self.alignmentMultiline = patch.align_multiline!.toTextAlignment()
        } else if patch.align_horizontal != nil {
            self.alignmentMultiline = patch.align_horizontal!.toTextAlignment()
        }
        if patch.align_vertical != nil && patch.align_horizontal != nil {
            self.alignment = toAlignment(horizontalAlignment: patch.align_horizontal!, verticalAlignment: patch.align_vertical!)
        }
        
        if patch.font_size != nil {
            self.font_size = patch.font_size!
        }
        
        if patch.underline != nil {
            self.underline = patch.underline!
        }
    }
}

public class TextElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var content: String
    public var editable: Bool
    public var clip: Bool
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var textStyle: TextStyle
    public var selectable: Bool
    public var markdown: Bool
    public var style_link: TextStyle?
    public var lastMeasuredSize: CGSize?
    public var nativeMaskPatch: NativeMaskPatch? = nil
    
    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, content: String, editable: Bool, clip: Bool, transform: [Float], size_x: Float, size_y: Float, opacity: Double, textStyle: TextStyle, selectable: Bool, markdown: Bool, style_link: TextStyle?, lastMeasuredSize: CGSize? = nil) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.content = content
        self.editable = editable
        self.clip = clip
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.textStyle = textStyle
        self.selectable = selectable
        self.markdown = markdown
        self.style_link = style_link
        self.lastMeasuredSize = lastMeasuredSize
    }
    
    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> TextElement {
        let defaultTextStyle = TextStyle(font: PaxFont.makeDefault(), fill: Color(.black), alignmentMultiline: .leading, alignment: .topLeading, font_size: 5.0, underline: false)
        return TextElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, content: "", editable: false, clip: false, transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0, opacity: 1.0, textStyle: defaultTextStyle, selectable: false, markdown: false, style_link: nil)
    }
    
    public func applyPatch(patch: TextUpdatePatch) {
        //no-op to ID, as it is primary key
        
        if let content = patch.content {
            self.content = content
        }
        if let editable = patch.editable {
            self.editable = editable
        }
        if let clip = patch.clip {
            self.clip = clip
        }
        if let transform = patch.transform {
            self.transform = transform
        }
        if let size_x = patch.size_x {
            self.size_x = size_x
        }
        if let size_y = patch.size_y {
            self.size_y = size_y
        }
        if let opacity = patch.opacity {
            self.opacity = opacity
        }
        if let selectable = patch.selectable {
            self.selectable = selectable
        }
        if let markdown = patch.markdown {
            self.markdown = markdown
        }
        
        // Apply new TextStyle
        if let styleBuffer = patch.style {
            self.textStyle.applyPatch(from: styleBuffer)
        }
        
        // Apply style_link
        if let styleLinkBuffer = patch.style_link {
            if self.style_link == nil {
                self.style_link = TextStyle(
                    font: PaxFont.makeDefault(),
                    fill: self.textStyle.fill,
                    alignmentMultiline: self.textStyle.alignmentMultiline,
                    alignment: self.textStyle.alignment,
                    font_size: self.textStyle.font_size,
                    underline: self.textStyle.underline
                )
            }
            self.style_link?.applyPatch(from: styleLinkBuffer)
        }
    }

}

public class MaskPathPatch {
    public var path: String
    public var clips: [String]
    public var opacity: Double

    public init(path: String, clips: [String], opacity: Double) {
        self.path = path
        self.clips = clips
        self.opacity = opacity
    }
}

public class NativeMaskPatch {
    public var id: PaxNodeId
    public var size_x: Float
    public var size_y: Float
    public var entries: [MaskPathPatch]

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.size_x = Float(readDouble(fb["size_x"]) ?? 0.0)
        self.size_y = Float(readDouble(fb["size_y"]) ?? 0.0)
        self.entries = readMaskEntries(fb["entries"]) ?? []
    }
}

public enum TextAlignHorizontal {
    case center
    case left
    case right
}

public extension TextAlignHorizontal {
    func toTextAlignment() -> TextAlignment {
        switch self {
        case .center:
            return .center
        case .left:
            return .leading
        case .right:
            return .trailing
        }
    }
}

public enum TextAlignVertical {
    case top
    case center
    case bottom
}


public func toAlignment(horizontalAlignment: TextAlignHorizontal, verticalAlignment: TextAlignVertical) -> Alignment {
    let horizontal: HorizontalAlignment
    let vertical: VerticalAlignment
    
    switch horizontalAlignment {
    case .center:
        horizontal = .center
    case .left:
        horizontal = .leading
    case .right:
        horizontal = .trailing
    }
    
    switch verticalAlignment {
    case .top:
        vertical = .top
    case .center:
        vertical = .center
    case .bottom:
        vertical = .bottom
    }
    return Alignment(horizontal: horizontal, vertical: vertical)
}


/// A patch representing an image load request from a given id_chain
public class ImageLoadPatch {
    public var id: PaxNodeId
    public var id_chain: [UInt64]
    public var path: String?
    
    public init(fb:FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? decodeId(fb) ?? 0
        self.id_chain = decodeIdChain(fb)
        self.path = fb["path"]?.asString
    }
}

public class ScreenshotPatch {
    public var id: UInt32
    public var scale: CGFloat?

    public init(fb: FlxbReference) {
        self.id = UInt32(fb["id"]!.asUInt64!)
        self.scale = fb["scale"]?.asFloat.map { CGFloat($0) }
    }
}


public class TextStyleMessage {
    public var font: FlxbReference
    public var fill: Color?
    public var font_size: CGFloat?
    public var underline: Bool?
    public var align_multiline: TextAlignHorizontal?
    public var align_horizontal: TextAlignHorizontal?
    public var align_vertical: TextAlignVertical?
    
    public init(_ buffer: FlxbReference) {
        self.font =  buffer["font"]!
        
        self.font_size = buffer["font_size"]?.asFloat.map { CGFloat($0) }
        self.underline = buffer["underline"]?.asBool
        
        if let alignmentValue = buffer["align_multiline"]?.asString {
            switch alignmentValue {
            case "Center":
                self.align_multiline = .center
            case "Left":
                self.align_multiline = .left
            case "Right":
                self.align_multiline = .right
            default:
                self.align_multiline = nil
            }
        }
        
        if let alignmentValue = buffer["align_horizontal"]?.asString {
            switch alignmentValue {
            case "Center":
                self.align_horizontal = .center
            case "Left":
                self.align_horizontal = .left
            case "Right":
                self.align_horizontal = .right
            default:
                self.align_horizontal = nil
            }
        }
        
        if let verticalAlignmentValue = buffer["align_vertical"]?.asString {
            switch verticalAlignmentValue {
            case "Top":
                self.align_vertical = .top
            case "Center":
                self.align_vertical = .center
            case "Bottom":
                self.align_vertical = .bottom
            default:
                self.align_vertical = nil
            }
        }
        
        if let colorBuffer = buffer["fill"], !colorBuffer.isNull {
            self.fill = extractColorFromBuffer(colorBuffer)
        }
    }
}


public class TextUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var id_chain: [UInt64]
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var content: String?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var editable: Bool?
    public var selectable: Bool?
    public var clip: Bool?
    public var markdown: Bool?
    public var style: TextStyleMessage?
    public var style_link: TextStyleMessage?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? decodeId(fb) ?? 0
        self.id_chain = decodeIdChain(fb)
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.content = fb["content"]?.asString
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.editable = fb["editable"]?.asBool
        self.selectable = fb["selectable"]?.asBool
        self.clip = fb["clip"]?.asBool
        self.markdown = fb["markdown"]?.asBool
        
        if let styleBuffer = fb["style"], !styleBuffer.isNull {
            self.style = TextStyleMessage(styleBuffer)
        }
        
        if let styleLinkBuffer = fb["style_link"], !styleLinkBuffer.isNull {
            self.style_link = TextStyleMessage(styleLinkBuffer)
        }
    }
}

///// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
//public class TextUpdatePatch {
//    public var id_chain: [UInt64]
//    public var content: String?
//    public var transform: [Float]?
//    public var size_x: Float?
//    public var size_y: Float?
//    public var fontBuffer: FlxbReference
//    public var fill: Color?
//    public var align_multiline: TextAlignHorizontal?
//    public var align_vertical: TextAlignVertical?
//    public var align_horizontal: TextAlignHorizontal?
//    // New properties
//    public var size: CGFloat?
//    public var style_link: LinkStyle?
//
//    init(fb: FlxbReference) {
//        self.id_chain = fb["id_chain"]!.asVector!.makeIterator().map({ fb in
//            fb.asUInt64!
//        })
//        self.content = fb["content"]?.asString
//        self.transform = fb["transform"]?.asVector?.makeIterator().map({ fb in
//            fb.asFloat!
//        })
//        self.size_x = fb["size_x"]?.asFloat
//        self.size_y = fb["size_y"]?.asFloat
//        self.fontBuffer =  fb["font"]!
//
//        if let fillBuffer = fb["fill"], !fillBuffer.isNull {
//            self.fill = extractColorFromBuffer(fillBuffer)
//        }
//
//        if let alignmentValue = fb["align_multiline"]?.asString {
//            switch alignmentValue {
//            case "Center":
//                self.align_multiline = .center
//            case "Left":
//                self.align_multiline = .left
//            case "Right":
//                self.align_multiline = .right
//            default:
//                self.align_multiline = nil
//            }
//        }
//
//        if let verticalAlignmentValue = fb["align_vertical"]?.asString {
//            switch verticalAlignmentValue {
//            case "Top":
//                self.align_vertical = .top
//            case "Center":
//                self.align_vertical = .center
//            case "Bottom":
//                self.align_vertical = .bottom
//            default:
//                self.align_vertical = nil
//            }
//        }
//
//        if let alignmentValue = fb["align_horizontal"]?.asString {
//            switch alignmentValue {
//            case "Center":
//                self.align_horizontal = .center
//            case "Left":
//                self.align_horizontal = .left
//            case "Right":
//                self.align_horizontal = .right
//            default:
//                self.align_horizontal = nil
//            }
//        }
//
//        self.size = fb["size"]?.asFloat.map { CGFloat($0) }
//
//        if !fb["style_link"]!.isNull {
//            self.style_link = LinkStyle(fb: fb["style_link"]!)
//        }
//
//    }
//}


public func extractColorFromBuffer(_ fillBuffer: FlxbReference) -> Color {
    if let rgba = fillBuffer["Rgba"], !rgba.isNull {
        let stub = fillBuffer["Rgba"]!
        return Color(
            red: readDouble(stub[0]) ?? 0.0,
            green: readDouble(stub[1]) ?? 0.0,
            blue: readDouble(stub[2]) ?? 0.0,
            opacity: readDouble(stub[3]) ?? 1.0
        )
    } else if let rgb = fillBuffer["Rgb"], !rgb.isNull {
        let stub = fillBuffer["Rgb"]!
        return Color(
            red: readDouble(stub[0]) ?? 0.0,
            green: readDouble(stub[1]) ?? 0.0,
            blue: readDouble(stub[2]) ?? 0.0,
            opacity: 1.0
        )
    } else {
        return Color.black
    }
}

public enum TextAlignHorizontalMessage: String {
    case Left, Center, Right
}

public enum FontStyle: String {
    case normal = "Normal"
    case italic = "Italic"
    case oblique = "Oblique"
}

extension FontWeight {
    public func fontWeight() -> Font.Weight {
        switch self {
        case .thin: return .thin
        case .extraLight: return .ultraLight
        case .light: return .light
        case .normal: return .regular
        case .medium: return .medium
        case .semiBold: return .semibold
        case .bold: return .bold
        case .extraBold: return .heavy
        case .black: return .black
        }
    }
}

#if os(iOS) || os(tvOS) || os(watchOS)
extension FontWeight {
    public func uiFontWeight() -> UIFont.Weight {
        switch self {
        case .thin: return .thin
        case .extraLight: return .ultraLight
        case .light: return .light
        case .normal: return .regular
        case .medium: return .medium
        case .semiBold: return .semibold
        case .bold: return .bold
        case .extraBold: return .heavy
        case .black: return .black
        }
    }
}
#elseif os(macOS)
extension FontWeight {
    public func nsFontWeight() -> NSFont.Weight {
        switch self {
        case .thin: return .thin
        case .extraLight: return .ultraLight
        case .light: return .light
        case .normal: return .regular
        case .medium: return .medium
        case .semiBold: return .semibold
        case .bold: return .bold
        case .extraBold: return .heavy
        case .black: return .black
        }
    }
}
#endif

public enum FontWeight: String {
    case thin = "Thin"
    case extraLight = "ExtraLight"
    case light = "Light"
    case normal = "Normal"
    case medium = "Medium"
    case semiBold = "SemiBold"
    case bold = "Bold"
    case extraBold = "ExtraBold"
    case black = "Black"
}

public class PaxFont {
    public enum PaxFontType {
        case system(SystemFont)
        case web(WebFont)
        case local(LocalFont)
    }

    public struct SystemFont {
        let family: String
        let style: FontStyle
        let weight: FontWeight
    }

    public struct WebFont {
        let family: String
        let url: URL
        let style: FontStyle
        let weight: FontWeight
    }

    public struct LocalFont {
        let family: String
        let path: URL
        let style: FontStyle
        let weight: FontWeight
    }

    public var type: PaxFontType
    public var cachedFont: Font?
    public var currentSize: CGFloat
    public var currentFontGeneration: UInt64
    #if os(iOS) || os(tvOS) || os(watchOS)
    public var cachedUIFont: UIFont?
    public var currentUIFontSize: CGFloat
    public var currentUIFontGeneration: UInt64
    #elseif os(macOS)
    public var cachedNSFont: NSFont?
    public var currentNSFontSize: CGFloat
    public var currentNSFontGeneration: UInt64
    #endif

    #if os(macOS)
    private static var registeredFontCache: [String: Bool] = [:]
    #elseif os(iOS) || os(tvOS) || os(watchOS)
    private static var registeredFontCache: [String: Bool] = [:]
    #endif
    private static var resolvedFontNameCache: [String: String] = [:]
    private static var fontRegistryGeneration: UInt64 = 0

    public init(type: PaxFontType) {
        self.type = type
        self.currentSize = 12
        self.currentFontGeneration = 0
        #if os(iOS) || os(tvOS) || os(watchOS)
        self.currentUIFontSize = 12
        self.currentUIFontGeneration = 0
        #elseif os(macOS)
        self.currentNSFontSize = 12
        self.currentNSFontGeneration = 0
        #endif
    }
    
    public static func makeDefault() -> PaxFont {
        let defaultSystemFont = SystemFont(family: "Helvetica", style: .normal, weight: .normal)
        return PaxFont(type: .system(defaultSystemFont))
    }

    private static func normalizedFontToken(_ value: String) -> String {
        value
            .lowercased()
            .unicodeScalars
            .filter { CharacterSet.alphanumerics.contains($0) }
            .map(String.init)
            .joined()
    }

    private static func weightKeywords(_ weight: FontWeight) -> [String] {
        switch weight {
        case .thin:
            return ["thin"]
        case .extraLight:
            return ["extralight", "ultralight"]
        case .light:
            return ["light"]
        case .normal:
            return ["regular", "romanregular", "book", "normal"]
        case .medium:
            return ["medium"]
        case .semiBold:
            return ["semibold", "demibold"]
        case .bold:
            return ["bold"]
        case .extraBold:
            return ["extrabold", "ultrabold"]
        case .black:
            return ["black", "heavy"]
        }
    }

    private static func inferredWeight(for normalizedCandidate: String) -> FontWeight? {
        if normalizedCandidate.contains("extrabold") || normalizedCandidate.contains("ultrabold") {
            return .extraBold
        }
        if normalizedCandidate.contains("semibold") || normalizedCandidate.contains("demibold") {
            return .semiBold
        }
        if normalizedCandidate.contains("black") || normalizedCandidate.contains("heavy") {
            return .black
        }
        if normalizedCandidate.contains("medium") {
            return .medium
        }
        if normalizedCandidate.contains("extralight") || normalizedCandidate.contains("ultralight") {
            return .extraLight
        }
        if normalizedCandidate.contains("light") {
            return .light
        }
        if normalizedCandidate.contains("thin") {
            return .thin
        }
        if normalizedCandidate.contains("regular")
            || normalizedCandidate.contains("romanregular")
            || normalizedCandidate.contains("book")
            || normalizedCandidate.contains("normal")
        {
            return .normal
        }
        if normalizedCandidate.contains("bold") {
            return .bold
        }
        return nil
    }

    private static func fontResolutionCacheKey(fontFamily: String, style: FontStyle, weight: FontWeight) -> String {
        "\(normalizedFontToken(fontFamily))|\(style.rawValue)|\(weight.rawValue)"
    }

    private static func candidateScore(
        candidateName: String,
        requestedFamily: String,
        style: FontStyle,
        weight: FontWeight,
        isFamilyName: Bool
    ) -> Int {
        let candidate = normalizedFontToken(candidateName)
        let requested = normalizedFontToken(requestedFamily)
        var score = 0

        if candidate == requested {
            score += 200
        }
        if candidate.hasPrefix(requested) || candidate.contains(requested) {
            score += 100
        }

        let expectsItalic = style == .italic || style == .oblique
        let isItalic = candidate.contains("italic") || candidate.contains("oblique")
        if expectsItalic {
            score += isItalic ? 60 : -20
        } else if isItalic {
            score -= 20
        }

        let keywords = weightKeywords(weight)
        if let inferredWeight = inferredWeight(for: candidate) {
            if inferredWeight == weight {
                score += 120
            } else {
                score -= 40
            }
        } else if keywords.contains(where: { candidate.contains($0) }) {
            score += 60
        } else if weight == .normal && candidate.contains("regular") {
            score += 40
        } else if weight != .normal && candidate.contains("regular") {
            score -= 10
        }

        if isFamilyName {
            score -= 120
        }

        return score
    }

    private static func resolveFontName(fontFamily: String, style: FontStyle, weight: FontWeight) -> String? {
        let cacheKey = fontResolutionCacheKey(fontFamily: fontFamily, style: style, weight: weight)
        if let cached = resolvedFontNameCache[cacheKey] {
            return cached.isEmpty ? nil : cached
        }

        let requested = normalizedFontToken(fontFamily)
        var candidates: [(name: String, isFamily: Bool)] = []
        var seen: Set<String> = []

        for registeredName in registeredFontCache.keys {
            let normalizedName = normalizedFontToken(registeredName)
            guard normalizedName == requested
                || normalizedName.contains(requested)
                || requested.contains(normalizedName)
            else {
                continue
            }
            if seen.insert(registeredName).inserted {
                candidates.append((registeredName, normalizedName == requested))
            }
        }

        #if os(iOS) || os(tvOS) || os(watchOS)
        for familyName in UIFont.familyNames {
            let normalizedFamily = normalizedFontToken(familyName)
            guard normalizedFamily == requested
                || normalizedFamily.contains(requested)
                || requested.contains(normalizedFamily)
            else {
                continue
            }
            if seen.insert(familyName).inserted {
                candidates.append((familyName, true))
            }
            for fontName in UIFont.fontNames(forFamilyName: familyName) {
                if seen.insert(fontName).inserted {
                    candidates.append((fontName, false))
                }
            }
        }
        #elseif os(macOS)
        for familyName in NSFontManager.shared.availableFontFamilies {
            let normalizedFamily = normalizedFontToken(familyName)
            guard normalizedFamily == requested
                || normalizedFamily.contains(requested)
                || requested.contains(normalizedFamily)
            else {
                continue
            }
            if seen.insert(familyName).inserted {
                candidates.append((familyName, true))
            }
            if let members = NSFontManager.shared.availableMembers(ofFontFamily: familyName) {
                for member in members {
                    if let postscriptName = member.first as? String, seen.insert(postscriptName).inserted {
                        candidates.append((postscriptName, false))
                    }
                }
            }
        }
        #endif

        let resolutionPool = candidates.contains(where: { !$0.isFamily })
            ? candidates.filter { !$0.isFamily }
            : candidates

        let resolved = resolutionPool.max {
            candidateScore(
                candidateName: $0.name,
                requestedFamily: fontFamily,
                style: style,
                weight: weight,
                isFamilyName: $0.isFamily
            ) < candidateScore(
                candidateName: $1.name,
                requestedFamily: fontFamily,
                style: style,
                weight: weight,
                isFamilyName: $1.isFamily
            )
        }?.name

        resolvedFontNameCache[cacheKey] = resolved ?? ""
        return resolved
    }

    private static func isFontAvailable(fontFamily: String, style: FontStyle, weight: FontWeight) -> Bool {
        if isFontRegistered(fontFamily: fontFamily) {
            return true
        }
        return resolveFontName(fontFamily: fontFamily, style: style, weight: weight) != nil
    }
    
    public func getFont(size: CGFloat) -> Font {
        ensureFontAvailabilityIfNeeded()
        let registryGeneration = PaxFont.fontRegistryGeneration
        if let cachedFont = cachedFont, currentSize == size, currentFontGeneration == registryGeneration {
            return cachedFont
        }
        
        var fontFamily: String?
        var fontStyle: FontStyle?
        var fontWeight: FontWeight?

        switch type {
        case .system(let systemFont):
            fontFamily = systemFont.family
            fontStyle = systemFont.style
            fontWeight = systemFont.weight
        case .web(let webFont):
            fontFamily = webFont.family
            fontStyle = webFont.style
            fontWeight = webFont.weight
        case .local(let localFont):
            fontFamily = localFont.family
            fontStyle = localFont.style
            fontWeight = localFont.weight
        }
        
        let resolvedFontName = PaxFont.resolveFontName(
            fontFamily: fontFamily!,
            style: fontStyle!,
            weight: fontWeight!
        )

        let baseFont: Font
        if let resolvedFontName {
            baseFont = Font.custom(resolvedFontName, size: size).weight(fontWeight!.fontWeight())
        } else {
            baseFont = .system(size: size).weight(fontWeight!.fontWeight())
        }

        let finalFont: Font
        switch fontStyle! {
        case .normal:
            finalFont = baseFont
        case .italic:
            finalFont = baseFont.italic()
        case .oblique:
            finalFont = baseFont
        }

        cachedFont = finalFont
        currentSize = size
        currentFontGeneration = registryGeneration

        return finalFont
    }

    #if os(iOS) || os(tvOS) || os(watchOS)
    public func getUIFont(size: CGFloat) -> UIFont {
        ensureFontAvailabilityIfNeeded()
        let registryGeneration = PaxFont.fontRegistryGeneration
        if let cachedUIFont = cachedUIFont, currentUIFontSize == size, currentUIFontGeneration == registryGeneration {
            return cachedUIFont
        }

        var fontFamily: String?
        var fontStyle: FontStyle?
        var fontWeight: FontWeight?

        switch type {
        case .system(let systemFont):
            fontFamily = systemFont.family
            fontStyle = systemFont.style
            fontWeight = systemFont.weight
        case .web(let webFont):
            fontFamily = webFont.family
            fontStyle = webFont.style
            fontWeight = webFont.weight
        case .local(let localFont):
            fontFamily = localFont.family
            fontStyle = localFont.style
            fontWeight = localFont.weight
        }

        let resolvedFontName = fontFamily.flatMap {
            PaxFont.resolveFontName(
                fontFamily: $0,
                style: fontStyle ?? .normal,
                weight: fontWeight ?? .normal
            )
        }

        let baseFont: UIFont
        if let resolvedFontName {
            baseFont = UIFont(name: resolvedFontName, size: size) ?? UIFont.systemFont(ofSize: size, weight: fontWeight?.uiFontWeight() ?? .regular)
        } else {
            baseFont = UIFont.systemFont(ofSize: size, weight: fontWeight?.uiFontWeight() ?? .regular)
        }

        let finalFont: UIFont
        switch fontStyle ?? .normal {
        case .normal:
            finalFont = baseFont
        case .italic:
            if let descriptor = baseFont.fontDescriptor.withSymbolicTraits(.traitItalic) {
                finalFont = UIFont(descriptor: descriptor, size: size)
            } else {
                finalFont = UIFont.italicSystemFont(ofSize: size)
            }
        case .oblique:
            finalFont = baseFont
        }
        cachedUIFont = finalFont
        currentUIFontSize = size
        currentUIFontGeneration = registryGeneration
        return finalFont
    }
    #elseif os(macOS)
    public func getNSFont(size: CGFloat) -> NSFont {
        ensureFontAvailabilityIfNeeded()
        let registryGeneration = PaxFont.fontRegistryGeneration
        if let cachedNSFont = cachedNSFont, currentNSFontSize == size, currentNSFontGeneration == registryGeneration {
            return cachedNSFont
        }

        var fontFamily: String?
        var fontStyle: FontStyle?
        var fontWeight: FontWeight?

        switch type {
        case .system(let systemFont):
            fontFamily = systemFont.family
            fontStyle = systemFont.style
            fontWeight = systemFont.weight
        case .web(let webFont):
            fontFamily = webFont.family
            fontStyle = webFont.style
            fontWeight = webFont.weight
        case .local(let localFont):
            fontFamily = localFont.family
            fontStyle = localFont.style
            fontWeight = localFont.weight
        }

        let resolvedFontName = fontFamily.flatMap {
            PaxFont.resolveFontName(
                fontFamily: $0,
                style: fontStyle ?? .normal,
                weight: fontWeight ?? .normal
            )
        }

        let baseFont: NSFont
        if let resolvedFontName {
            baseFont = NSFont(name: resolvedFontName, size: size) ?? NSFont.systemFont(ofSize: size, weight: fontWeight?.nsFontWeight() ?? .regular)
        } else {
            baseFont = NSFont.systemFont(ofSize: size, weight: fontWeight?.nsFontWeight() ?? .regular)
        }

        let finalFont: NSFont
        switch fontStyle ?? .normal {
        case .normal:
            finalFont = baseFont
        case .italic:
            let descriptor = baseFont.fontDescriptor.withSymbolicTraits(.italic)
            finalFont = NSFont(descriptor: descriptor, size: size) ?? NSFontManager.shared.convert(baseFont, toHaveTrait: .italicFontMask)
        case .oblique:
            finalFont = baseFont
        }
        cachedNSFont = finalFont
        currentNSFontSize = size
        currentNSFontGeneration = registryGeneration
        return finalFont
    }
    #endif



    public func applyPatch(fb: FlxbReference) {
        cachedFont = nil
        currentFontGeneration = 0
        #if os(iOS) || os(tvOS) || os(watchOS)
        cachedUIFont = nil
        currentUIFontGeneration = 0
        #elseif os(macOS)
        cachedNSFont = nil
        currentNSFontGeneration = 0
        #endif
        if let systemFontMessage = fb["System"] {
            if let family = systemFontMessage["family"]?.asString {
                let styleMessage = FontStyle(rawValue: systemFontMessage["style"]?.asString ?? "normal") ?? .normal
                let weightMessage = FontWeight(rawValue: systemFontMessage["weight"]?.asString ?? "normal") ?? .normal
                self.type = .system(SystemFont(family: family, style: styleMessage, weight: weightMessage))
            }
        } else if let webFontMessage = fb["Web"] {
            if let family = webFontMessage["family"]?.asString,
               let urlString = webFontMessage["url"]?.asString,
               let url = URL(string: urlString) {
                let style = FontStyle(rawValue: webFontMessage["style"]?.asString ?? "normal") ?? .normal
                let weight = FontWeight(rawValue: webFontMessage["weight"]?.asString ?? "normal") ?? .normal

                self.type = .web(WebFont(family: family, url: url, style: style, weight: weight))
            }
        } else if let localFontMessage = fb["Local"] {
            if let family = localFontMessage["family"]?.asString,
               let pathString = localFontMessage["path"]?.asString,
               let path = URL(string: pathString) {
                let style = FontStyle(rawValue: localFontMessage["style"]?.asString ?? "normal") ?? .normal
                let weight = FontWeight(rawValue: localFontMessage["weight"]?.asString ?? "normal") ?? .normal

                self.type = .local(LocalFont(family: family, path: path, style: style, weight: weight))
            }
        }
    }

    private func ensureFontAvailabilityIfNeeded() {
        switch type {
        case .web(let webFont):
            if !PaxFont.isFontAvailable(
                fontFamily: webFont.family,
                style: webFont.style,
                weight: webFont.weight
            ) {
                PaxWebFontLoader.shared.ensureLoaded(font: webFont)
            }
        case .local(let localFont):
            if !PaxFont.isFontAvailable(
                fontFamily: localFont.family,
                style: localFont.style,
                weight: localFont.weight
            ) {
                var errorRef: Unmanaged<CFError>?
                if CTFontManagerRegisterFontsForURL(localFont.path as CFURL, .process, &errorRef) {
                    PaxFont.markFontRegistered(fontFamily: localFont.family)
                }
            }
        case .system:
            break
        }
    }

    #if os(macOS)
    public static func isFontRegistered(fontFamily: String) -> Bool {
        if let cached = registeredFontCache[fontFamily] {
            return cached
        }
        let fontFamilies = CTFontManagerCopyAvailableFontFamilyNames() as! [String]

        if fontFamilies.contains(fontFamily) {
            registeredFontCache[fontFamily] = true
            return true
        }

        // Check if the font is installed on the system using CTFontManager
        let installedFontURLs = CTFontManagerCopyAvailableFontURLs() as? [URL] ?? []

        for url in installedFontURLs {
            if let fontDescriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] {
                for descriptor in fontDescriptors {
                    if let fontFamilyName = CTFontDescriptorCopyAttribute(descriptor, kCTFontFamilyNameAttribute) as? String {
                        if fontFamilyName == fontFamily {
                            registeredFontCache[fontFamily] = true
                            return true
                        }
                    }
                }
            }
        }
        registeredFontCache[fontFamily] = false
        return false

    }
    @discardableResult
    public static func markFontRegistered(fontFamily: String) -> Bool {
        if registeredFontCache[fontFamily] == true {
            return false
        }
        registeredFontCache[fontFamily] = true
        resolvedFontNameCache.removeAll(keepingCapacity: true)
        fontRegistryGeneration &+= 1
        return true
    }
    #elseif  os(iOS) || os(tvOS) || os(watchOS)
    public static func isFontRegistered(fontFamily: String) -> Bool {
        if let cached = registeredFontCache[fontFamily] {
            return cached
        }
        let availableFontFamilies = UIFont.familyNames
        let isRegistered = availableFontFamilies.contains(fontFamily)
        registeredFontCache[fontFamily] = isRegistered
        return isRegistered
    }
    @discardableResult
    public static func markFontRegistered(fontFamily: String) -> Bool {
        if registeredFontCache[fontFamily] == true {
            return false
        }
        registeredFontCache[fontFamily] = true
        resolvedFontNameCache.removeAll(keepingCapacity: true)
        fontRegistryGeneration &+= 1
        return true
    }
    #endif
}
//
//public class FontFactory {
////    public var family: String
////    public var public variant: String
////    public var size: Float
//
//    func applyPatch(fb: FlxbReference) -> Font {
//        print("MAKING FONT")
//        print(fb.debugDescription)
//
//
//
//        var suffix = ""
//        if fb["variant"] != nil && !fb["variant"]!.isNull { && fb["variant"]!.asString! != "Regular" {
//            suffix = " " + fb["variant"]!.asString!
//        }
//        return Font.custom(String(fb["family"]!.asString! + suffix), size: CGFloat(fb["size"]!.asFloat!))
//    }
//
//
//    static func makeDefault() -> Font {
//        return Font.custom("Courier New", size: 14)
////        Font()
////        return Font(family: "Courier New", variant: "Regular", size: 14)
//    }
//}

//func registerWebFont() {
//       if case let .web(webFont) = type, !Self.isFontRegistered(fontFamily: webFont.family) {
//           URLSession.shared.dataTask(with: webFont.url) { data, response, error in
//               guard let data = data, error == nil else {
//                   print("Error downloading font: \(String(describing: error))")
//                   return
//               }
//               guard let provider = CGDataProvider(data: data as CFData) else {
//                   print("Error creating font provider")
//                   return
//               }
//               guard let font = CGFont(provider) else {
//                   print("Error creating font from data")
//                   return
//               }
//               print(font.fullName)
//               var errorRef: Unmanaged<CFError>?
//               if !CTFontManagerRegisterGraphicsFont(font, &errorRef) {
//                   print("Error registering font: \(webFont.family) - \(String(describing: errorRef))")
//               }
//           }.resume()
//       }
//   }



public class FrameElement: ResolvedPlacementTarget {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var clipContent: Bool
    public var opacity: Double
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var clipPath: String?
    public var borderRadius: Double
    
    public init(id: PaxNodeId, parentFrame: PaxNodeId?, clipContent: Bool, opacity: Double, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, clipPath: String? = nil, borderRadius: Double = 0.0) {
        self.id = id
        self.parentFrame = parentFrame
        self.clipContent = clipContent
        self.opacity = opacity
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.clipPath = clipPath
        self.borderRadius = borderRadius
    }
    
    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?) -> FrameElement {
        FrameElement(id: id, parentFrame: parentFrame, clipContent: false, opacity: 1.0, zIndex: 0, transform: [1,0,0,1,0,0], size_x: 0.0, size_y: 0.0)
    }
    
    public func applyPatch(patch: FrameUpdatePatch) {
        //no-op to ID, as it is primary key
        
        if patch.transform != nil {
            self.transform = patch.transform!
        }
        if patch.size_x != nil {
            self.size_x = patch.size_x!
        }
        if patch.size_y != nil {
            self.size_y = patch.size_y!
        }
        if let clipContent = patch.clipContent {
            self.clipContent = clipContent
        }
        if let opacity = patch.opacity {
            self.opacity = opacity
        }
        if let clipPath = patch.clipPath {
            self.clipPath = clipPath.isEmpty ? nil : clipPath
        }
        if let borderRadius = patch.borderRadius {
            self.borderRadius = borderRadius
        }
    }

}

public class ScrollerElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var clipContent: Bool
    public var borderRadius: Double
    public var sizeInnerPaneX: Float
    public var sizeInnerPaneY: Float
    public var snapPointsX: [Double]
    public var snapPointsY: [Double]
    public var scrollX: Double
    public var scrollY: Double
    public var presentationScrollX: Double
    public var presentationScrollY: Double
    public var scrollEnabledX: Bool
    public var scrollEnabledY: Bool
    public var contentLayerId: UInt32?
    public var presentedBounds: [Double]?
    public var presentedClipBounds: [Double]?
    public var subtreeDepth: UInt32
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(
        id: PaxNodeId,
        parentFrame: PaxNodeId?,
        renderLayerId: UInt32,
        zIndex: Int,
        transform: [Float],
        size_x: Float,
        size_y: Float,
        opacity: Double,
        clipContent: Bool,
        borderRadius: Double,
        sizeInnerPaneX: Float,
        sizeInnerPaneY: Float,
        snapPointsX: [Double],
        snapPointsY: [Double],
        scrollX: Double,
        scrollY: Double,
        presentationScrollX: Double,
        presentationScrollY: Double,
        scrollEnabledX: Bool,
        scrollEnabledY: Bool,
        contentLayerId: UInt32?,
        presentedBounds: [Double]?,
        presentedClipBounds: [Double]?,
        subtreeDepth: UInt32
    ) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.clipContent = clipContent
        self.borderRadius = borderRadius
        self.sizeInnerPaneX = sizeInnerPaneX
        self.sizeInnerPaneY = sizeInnerPaneY
        self.snapPointsX = snapPointsX
        self.snapPointsY = snapPointsY
        self.scrollX = scrollX
        self.scrollY = scrollY
        self.presentationScrollX = presentationScrollX
        self.presentationScrollY = presentationScrollY
        self.scrollEnabledX = scrollEnabledX
        self.scrollEnabledY = scrollEnabledY
        self.contentLayerId = contentLayerId
        self.presentedBounds = presentedBounds
        self.presentedClipBounds = presentedClipBounds
        self.subtreeDepth = subtreeDepth
    }

    public static func makeDefault(
        id: PaxNodeId,
        parentFrame: PaxNodeId?,
        renderLayerId: UInt32
    ) -> ScrollerElement {
        ScrollerElement(
            id: id,
            parentFrame: parentFrame,
            renderLayerId: renderLayerId,
            zIndex: 0,
            transform: [1, 0, 0, 1, 0, 0],
            size_x: 0,
            size_y: 0,
            opacity: 1.0,
            clipContent: true,
            borderRadius: 0.0,
            sizeInnerPaneX: 0,
            sizeInnerPaneY: 0,
            snapPointsX: [],
            snapPointsY: [],
            scrollX: 0,
            scrollY: 0,
            presentationScrollX: Double.nan,
            presentationScrollY: Double.nan,
            scrollEnabledX: true,
            scrollEnabledY: true,
            contentLayerId: nil,
            presentedBounds: nil,
            presentedClipBounds: nil,
            subtreeDepth: 0
        )
    }

    public func applyPatch(patch: ScrollerUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let clipContent = patch.clipContent { self.clipContent = clipContent }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let sizeInnerPaneX = patch.size_inner_pane_x { self.sizeInnerPaneX = sizeInnerPaneX }
        if let sizeInnerPaneY = patch.size_inner_pane_y { self.sizeInnerPaneY = sizeInnerPaneY }
        if let snapPointsX = patch.snap_points_x { self.snapPointsX = snapPointsX }
        if let snapPointsY = patch.snap_points_y { self.snapPointsY = snapPointsY }
        if let scrollX = patch.scroll_x { self.scrollX = scrollX }
        if let scrollY = patch.scroll_y { self.scrollY = scrollY }
        if let presentationScrollX = patch.presentation_scroll_x { self.presentationScrollX = presentationScrollX }
        if let presentationScrollY = patch.presentation_scroll_y { self.presentationScrollY = presentationScrollY }
        if let scrollEnabledX = patch.scroll_enabled_x { self.scrollEnabledX = scrollEnabledX }
        if let scrollEnabledY = patch.scroll_enabled_y { self.scrollEnabledY = scrollEnabledY }
        if let contentLayerId = patch.content_layer_id { self.contentLayerId = contentLayerId }
        if let presentedBounds = patch.presented_bounds { self.presentedBounds = presentedBounds }
        if let presentedClipBounds = patch.presented_clip_bounds { self.presentedClipBounds = presentedClipBounds }
        if let subtreeDepth = patch.subtree_depth { self.subtreeDepth = subtreeDepth }
    }
}



/// A patch containing optional fields, representing an update action for the NativeElement of the given id_chain
public class FrameUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var id_chain: [UInt64]
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var clipContent: Bool?
    public var clipPath: String?
    public var borderRadius: Double?
    public var opacity: Double?
    
    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? decodeId(fb) ?? 0
        self.id_chain = decodeIdChain(fb)
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.clipContent = fb["clip_content"]?.asBool
        self.clipPath = fb["clip_path"]?.asString
        self.borderRadius = readDouble(fb["border_radius"])
        self.opacity = readDouble(fb["opacity"])
    }
}

public class ScrollerUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var id_chain: [UInt64]
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var clipContent: Bool?
    public var borderRadius: Double?
    public var size_inner_pane_x: Float?
    public var size_inner_pane_y: Float?
    public var snap_points_x: [Double]?
    public var snap_points_y: [Double]?
    public var scroll_x: Double?
    public var scroll_y: Double?
    public var presentation_scroll_x: Double?
    public var presentation_scroll_y: Double?
    public var scroll_enabled_x: Bool?
    public var scroll_enabled_y: Bool?
    public var content_layer_id: UInt32?
    public var presented_bounds: [Double]?
    public var presented_clip_bounds: [Double]?
    public var subtree_depth: UInt32?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? decodeId(fb) ?? 0
        self.id_chain = decodeIdChain(fb)
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.clipContent = fb["clip_content"]?.asBool
        self.borderRadius = readDouble(fb["border_radius"])
        self.size_inner_pane_x = fb["size_inner_pane_x"]?.asFloat
        self.size_inner_pane_y = fb["size_inner_pane_y"]?.asFloat
        self.snap_points_x = readDoubleArray(fb["snap_points_x"])
        self.snap_points_y = readDoubleArray(fb["snap_points_y"])
        self.scroll_x = readDouble(fb["scroll_x"])
        self.scroll_y = readDouble(fb["scroll_y"])
        self.presentation_scroll_x = readDouble(fb["presentation_scroll_x"])
        self.presentation_scroll_y = readDouble(fb["presentation_scroll_y"])
        self.scroll_enabled_x = fb["scroll_enabled_x"]?.asBool
        self.scroll_enabled_y = fb["scroll_enabled_y"]?.asBool
        if let value = fb["content_layer_id"]?.asUInt64 {
            self.content_layer_id = UInt32(truncatingIfNeeded: value)
        } else {
            self.content_layer_id = nil
        }
        self.presented_bounds = readDoubleArray(fb["presented_bounds"])
        self.presented_clip_bounds = readDoubleArray(fb["presented_clip_bounds"])
        if let value = fb["subtree_depth"]?.asUInt64 {
            self.subtree_depth = UInt32(truncatingIfNeeded: value)
        } else {
            self.subtree_depth = nil
        }
    }
}

public func defaultPaxTextStyle() -> TextStyle {
    TextStyle(
        font: PaxFont.makeDefault(),
        fill: Color(.black),
        alignmentMultiline: .leading,
        alignment: .topLeading,
        font_size: 14.0,
        underline: false
    )
}

public protocol NativeMaskableElement: AnyObject {
    var nativeMaskPatch: NativeMaskPatch? { get set }
}

public extension NativeMaskableElement {
    func applyNativeMaskPatch(_ patch: NativeMaskPatch) {
        self.nativeMaskPatch = patch.entries.isEmpty ? nil : patch
    }
}

public protocol ResolvedPlacementPatch {
    var parentFrameUpdated: Bool { get }
    var parentFrame: PaxNodeId? { get }
    var zIndexUpdated: Bool { get }
    var zIndex: Int? { get }
}

public protocol ResolvedPlacementTarget: AnyObject {
    var parentFrame: PaxNodeId? { get set }
    var zIndex: Int { get set }
}

public extension ResolvedPlacementTarget {
    func applyResolvedPlacement<P: ResolvedPlacementPatch>(_ patch: P) {
        if patch.parentFrameUpdated {
            self.parentFrame = patch.parentFrame
        }
        if patch.zIndexUpdated, let zIndex = patch.zIndex {
            self.zIndex = zIndex
        }
    }
}

public protocol NativePositionElement: NativeMaskableElement, ResolvedPlacementTarget {
    var id: PaxNodeId { get }
    var parentFrame: PaxNodeId? { get set }
    var renderLayerId: UInt32 { get set }
    var zIndex: Int { get set }
    var transform: [Float] { get set }
    var size_x: Float { get set }
    var size_y: Float { get set }
    var opacity: Double { get set }
}

public class EventBlockerPatchMessage: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
    }
}

public class EventBlockerElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> EventBlockerElement {
        EventBlockerElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0)
    }

    public func applyPatch(_ patch: EventBlockerPatchMessage) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
    }
}

public class ButtonUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var hoverColor: Color?
    public var outlineStrokeColor: Color?
    public var outlineStrokeWidth: Double?
    public var borderRadius: Double?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var content: String?
    public var color: Color?
    public var style: TextStyleMessage?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.outlineStrokeWidth = readDouble(fb["outline_stroke_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.content = fb["content"]?.asString
        if let hoverColor = fb["hover_color"], !hoverColor.isNull {
            self.hoverColor = extractColorFromBuffer(hoverColor)
        }
        if let outlineStrokeColor = fb["outline_stroke_color"], !outlineStrokeColor.isNull {
            self.outlineStrokeColor = extractColorFromBuffer(outlineStrokeColor)
        }
        if let color = fb["color"], !color.isNull {
            self.color = extractColorFromBuffer(color)
        }
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
    }
}

public class ButtonElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var hoverColor: Color
    public var outlineStrokeColor: Color
    public var outlineStrokeWidth: Double
    public var borderRadius: Double
    public var content: String
    public var color: Color
    public var style: TextStyle
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, hoverColor: Color, outlineStrokeColor: Color, outlineStrokeWidth: Double, borderRadius: Double, content: String, color: Color, style: TextStyle) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.hoverColor = hoverColor
        self.outlineStrokeColor = outlineStrokeColor
        self.outlineStrokeWidth = outlineStrokeWidth
        self.borderRadius = borderRadius
        self.content = content
        self.color = color
        self.style = style
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> ButtonElement {
        ButtonElement(
            id: id,
            parentFrame: parentFrame,
            renderLayerId: renderLayerId,
            zIndex: 0,
            transform: [1, 0, 0, 1, 0, 0],
            size_x: 0,
            size_y: 0,
            opacity: 1.0,
            hoverColor: Color(.gray),
            outlineStrokeColor: Color(.clear),
            outlineStrokeWidth: 0,
            borderRadius: 8,
            content: "",
            color: Color(.gray),
            style: defaultPaxTextStyle()
        )
    }

    public func applyPatch(_ patch: ButtonUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let hoverColor = patch.hoverColor { self.hoverColor = hoverColor }
        if let outlineStrokeColor = patch.outlineStrokeColor { self.outlineStrokeColor = outlineStrokeColor }
        if let outlineStrokeWidth = patch.outlineStrokeWidth { self.outlineStrokeWidth = outlineStrokeWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let content = patch.content { self.content = content }
        if let color = patch.color { self.color = color }
        if let style = patch.style { self.style.applyPatch(from: style) }
    }
}

public class PhotoPickerUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var trigger: UInt64?
    public var source: String?
    public var allowMultiple: Bool?
    public var accept: String?
    public var includeBytes: Bool?
    public var maxBytesPerPhoto: UInt64?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        if let trigger = fb["trigger"]?.asUInt64 {
            self.trigger = trigger
        }
        self.source = fb["source"]?.asString
        self.allowMultiple = fb["allow_multiple"]?.asBool
        self.accept = fb["accept"]?.asString
        self.includeBytes = fb["include_bytes"]?.asBool
        if let maxBytes = fb["max_bytes_per_photo"]?.asUInt64 {
            self.maxBytesPerPhoto = maxBytes
        }
    }
}

public class PhotoPickerElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var occlusionLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var trigger: UInt64
    public var source: String
    public var allowMultiple: Bool
    public var accept: String
    public var includeBytes: Bool
    public var maxBytesPerPhoto: UInt64
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(
        id: PaxNodeId,
        parentFrame: PaxNodeId?,
        occlusionLayerId: UInt32,
        zIndex: Int,
        transform: [Float],
        size_x: Float,
        size_y: Float,
        opacity: Double,
        trigger: UInt64,
        source: String,
        allowMultiple: Bool,
        accept: String,
        includeBytes: Bool,
        maxBytesPerPhoto: UInt64
    ) {
        self.id = id
        self.parentFrame = parentFrame
        self.occlusionLayerId = occlusionLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.trigger = trigger
        self.source = source
        self.allowMultiple = allowMultiple
        self.accept = accept
        self.includeBytes = includeBytes
        self.maxBytesPerPhoto = maxBytesPerPhoto
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, occlusionLayerId: UInt32) -> PhotoPickerElement {
        PhotoPickerElement(
            id: id,
            parentFrame: parentFrame,
            occlusionLayerId: occlusionLayerId,
            zIndex: 0,
            transform: [1, 0, 0, 1, 0, 0],
            size_x: 0,
            size_y: 0,
            opacity: 1.0,
            trigger: 0,
            source: "library",
            allowMultiple: true,
            accept: "image/*",
            includeBytes: true,
            maxBytesPerPhoto: 25 * 1024 * 1024
        )
    }

    public func applyPatch(_ patch: PhotoPickerUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let trigger = patch.trigger { self.trigger = trigger }
        if let source = patch.source { self.source = source }
        if let allowMultiple = patch.allowMultiple { self.allowMultiple = allowMultiple }
        if let accept = patch.accept { self.accept = accept }
        if let includeBytes = patch.includeBytes { self.includeBytes = includeBytes }
        if let maxBytesPerPhoto = patch.maxBytesPerPhoto { self.maxBytesPerPhoto = maxBytesPerPhoto }
    }
}

public class CheckboxUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var background: Color?
    public var backgroundChecked: Color?
    public var outlineColor: Color?
    public var outlineWidth: Double?
    public var borderRadius: Double?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var checked: Bool?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.outlineWidth = readDouble(fb["outline_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.checked = fb["checked"]?.asBool
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
        if let backgroundChecked = fb["background_checked"], !backgroundChecked.isNull {
            self.backgroundChecked = extractColorFromBuffer(backgroundChecked)
        }
        if let outlineColor = fb["outline_color"], !outlineColor.isNull {
            self.outlineColor = extractColorFromBuffer(outlineColor)
        }
    }
}

public class CheckboxElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var background: Color
    public var backgroundChecked: Color
    public var outlineColor: Color
    public var outlineWidth: Double
    public var borderRadius: Double
    public var checked: Bool
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, background: Color, backgroundChecked: Color, outlineColor: Color, outlineWidth: Double, borderRadius: Double, checked: Bool) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.background = background
        self.backgroundChecked = backgroundChecked
        self.outlineColor = outlineColor
        self.outlineWidth = outlineWidth
        self.borderRadius = borderRadius
        self.checked = checked
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> CheckboxElement {
        CheckboxElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, background: Color(.white), backgroundChecked: Color(.blue), outlineColor: Color(.gray), outlineWidth: 1, borderRadius: 5, checked: false)
    }

    public func applyPatch(_ patch: CheckboxUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let background = patch.background { self.background = background }
        if let backgroundChecked = patch.backgroundChecked { self.backgroundChecked = backgroundChecked }
        if let outlineColor = patch.outlineColor { self.outlineColor = outlineColor }
        if let outlineWidth = patch.outlineWidth { self.outlineWidth = outlineWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let checked = patch.checked { self.checked = checked }
    }
}

public class NativeImageUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var url: String?
    public var fit: String?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.url = fb["url"]?.asString
        self.fit = fb["fit"]?.asString
    }
}

public class NativeImageElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var url: String
    public var fit: String
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, url: String, fit: String) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.url = url
        self.fit = fit
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> NativeImageElement {
        NativeImageElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, url: "", fit: "contain")
    }

    public func applyPatch(_ patch: NativeImageUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let url = patch.url { self.url = url }
        if let fit = patch.fit { self.fit = fit }
    }
}

public class YoutubeVideoUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var url: String?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.url = fb["url"]?.asString
    }
}

public class YoutubeVideoElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var url: String
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, url: String) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.url = url
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> YoutubeVideoElement {
        YoutubeVideoElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, url: "")
    }

    public func applyPatch(_ patch: YoutubeVideoUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let url = patch.url { self.url = url }
    }
}

public class DropdownUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var selectedId: UInt32?
    public var options: [String]?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var background: Color?
    public var strokeColor: Color?
    public var strokeWidth: Double?
    public var borderRadius: Double?
    public var style: TextStyleMessage?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.selectedId = readNodeId(fb["selected_id"])
        self.options = readStringArray(fb["options"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.strokeWidth = readDouble(fb["stroke_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
        if let strokeColor = fb["stroke_color"], !strokeColor.isNull {
            self.strokeColor = extractColorFromBuffer(strokeColor)
        }
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
    }
}

public class DropdownElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var selectedId: UInt32
    public var options: [String]
    public var background: Color
    public var strokeColor: Color
    public var strokeWidth: Double
    public var borderRadius: Double
    public var style: TextStyle
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, selectedId: UInt32, options: [String], background: Color, strokeColor: Color, strokeWidth: Double, borderRadius: Double, style: TextStyle) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.selectedId = selectedId
        self.options = options
        self.background = background
        self.strokeColor = strokeColor
        self.strokeWidth = strokeWidth
        self.borderRadius = borderRadius
        self.style = style
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> DropdownElement {
        DropdownElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, selectedId: 0, options: [], background: Color(.white), strokeColor: Color(.gray), strokeWidth: 1, borderRadius: 8, style: defaultPaxTextStyle())
    }

    public func applyPatch(_ patch: DropdownUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let selectedId = patch.selectedId { self.selectedId = selectedId }
        if let options = patch.options { self.options = options }
        if let background = patch.background { self.background = background }
        if let strokeColor = patch.strokeColor { self.strokeColor = strokeColor }
        if let strokeWidth = patch.strokeWidth { self.strokeWidth = strokeWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let style = patch.style { self.style.applyPatch(from: style) }
    }
}

public class RadioListUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var selectedId: UInt32?
    public var options: [String]?
    public var style: TextStyleMessage?
    public var backgroundChecked: Color?
    public var outlineColor: Color?
    public var outlineWidth: Double?
    public var background: Color?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.selectedId = readNodeId(fb["selected_id"])
        self.options = readStringArray(fb["options"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.outlineWidth = readDouble(fb["outline_width"])
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
        if let backgroundChecked = fb["background_checked"], !backgroundChecked.isNull {
            self.backgroundChecked = extractColorFromBuffer(backgroundChecked)
        }
        if let outlineColor = fb["outline_color"], !outlineColor.isNull {
            self.outlineColor = extractColorFromBuffer(outlineColor)
        }
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
    }
}

public class RadioListElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var selectedId: UInt32
    public var options: [String]
    public var style: TextStyle
    public var backgroundChecked: Color
    public var outlineColor: Color
    public var outlineWidth: Double
    public var background: Color
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, selectedId: UInt32, options: [String], style: TextStyle, backgroundChecked: Color, outlineColor: Color, outlineWidth: Double, background: Color) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.selectedId = selectedId
        self.options = options
        self.style = style
        self.backgroundChecked = backgroundChecked
        self.outlineColor = outlineColor
        self.outlineWidth = outlineWidth
        self.background = background
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> RadioListElement {
        RadioListElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, selectedId: 0, options: [], style: defaultPaxTextStyle(), backgroundChecked: Color(.blue), outlineColor: Color(.gray), outlineWidth: 1, background: Color(.white))
    }

    public func applyPatch(_ patch: RadioListUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let selectedId = patch.selectedId { self.selectedId = selectedId }
        if let options = patch.options { self.options = options }
        if let backgroundChecked = patch.backgroundChecked { self.backgroundChecked = backgroundChecked }
        if let outlineColor = patch.outlineColor { self.outlineColor = outlineColor }
        if let outlineWidth = patch.outlineWidth { self.outlineWidth = outlineWidth }
        if let background = patch.background { self.background = background }
        if let style = patch.style { self.style.applyPatch(from: style) }
    }
}

public class SliderUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var value: Double?
    public var step: Double?
    public var min: Double?
    public var max: Double?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var accent: Color?
    public var background: Color?
    public var borderRadius: Double?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.value = readDouble(fb["value"])
        self.step = readDouble(fb["step"])
        self.min = readDouble(fb["min"])
        self.max = readDouble(fb["max"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.borderRadius = readDouble(fb["border_radius"])
        if let accent = fb["accent"], !accent.isNull {
            self.accent = extractColorFromBuffer(accent)
        }
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
    }
}

public class SliderElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var value: Double
    public var step: Double
    public var min: Double
    public var max: Double
    public var accent: Color
    public var background: Color
    public var borderRadius: Double
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, value: Double, step: Double, min: Double, max: Double, accent: Color, background: Color, borderRadius: Double) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.value = value
        self.step = step
        self.min = min
        self.max = max
        self.accent = accent
        self.background = background
        self.borderRadius = borderRadius
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> SliderElement {
        SliderElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, value: 0, step: 1, min: 0, max: 100, accent: Color(.blue), background: Color(.gray), borderRadius: 5)
    }

    public func applyPatch(_ patch: SliderUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let value = patch.value { self.value = value }
        if let step = patch.step { self.step = step }
        if let min = patch.min { self.min = min }
        if let max = patch.max { self.max = max }
        if let accent = patch.accent { self.accent = accent }
        if let background = patch.background { self.background = background }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
    }
}

public class TextboxUpdatePatch: ResolvedPlacementPatch {
    public var id: PaxNodeId
    public var parentFrameUpdated: Bool
    public var parentFrame: PaxNodeId?
    public var zIndexUpdated: Bool
    public var zIndex: Int?
    public var transform: [Float]?
    public var size_x: Float?
    public var size_y: Float?
    public var opacity: Double?
    public var text: String?
    public var background: Color?
    public var strokeColor: Color?
    public var strokeWidth: Double?
    public var borderRadius: Double?
    public var style: TextStyleMessage?
    public var focusOnMount: Bool?
    public var placeholder: String?
    public var outlineColor: Color?
    public var outlineWidth: Double?
    public var isTextArea: Bool?

    public init(fb: FlxbReference) {
        self.id = readNodeId(fb["id"]) ?? 0
        self.parentFrameUpdated = fieldExists(fb, "parent_frame")
        self.parentFrame = readNodeId(fb["parent_frame"])
        self.zIndexUpdated = fieldExists(fb, "z_index")
        self.zIndex = readInt(fb["z_index"])
        self.transform = readFloatArray(fb["transform"])
        self.size_x = fb["size_x"]?.asFloat
        self.size_y = fb["size_y"]?.asFloat
        self.opacity = readDouble(fb["opacity"])
        self.text = fb["text"]?.asString
        self.strokeWidth = readDouble(fb["stroke_width"])
        self.borderRadius = readDouble(fb["border_radius"])
        self.focusOnMount = fb["focus_on_mount"]?.asBool
        self.placeholder = fb["placeholder"]?.asString
        self.outlineWidth = readDouble(fb["outline_width"])
        self.isTextArea = fb["is_text_area"]?.asBool
        if let background = fb["background"], !background.isNull {
            self.background = extractColorFromBuffer(background)
        }
        if let strokeColor = fb["stroke_color"], !strokeColor.isNull {
            self.strokeColor = extractColorFromBuffer(strokeColor)
        }
        if let outlineColor = fb["outline_color"], !outlineColor.isNull {
            self.outlineColor = extractColorFromBuffer(outlineColor)
        }
        if let style = fb["style"], !style.isNull {
            self.style = TextStyleMessage(style)
        }
    }
}

public class TextboxElement: NativePositionElement {
    public var id: PaxNodeId
    public var parentFrame: PaxNodeId?
    public var renderLayerId: UInt32
    public var zIndex: Int
    public var transform: [Float]
    public var size_x: Float
    public var size_y: Float
    public var opacity: Double
    public var text: String
    public var background: Color
    public var strokeColor: Color
    public var strokeWidth: Double
    public var borderRadius: Double
    public var style: TextStyle
    public var focusOnMount: Bool
    public var placeholder: String
    public var outlineColor: Color
    public var outlineWidth: Double
    public var isTextArea: Bool
    public var nativeMaskPatch: NativeMaskPatch? = nil

    public init(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32, zIndex: Int, transform: [Float], size_x: Float, size_y: Float, opacity: Double, text: String, background: Color, strokeColor: Color, strokeWidth: Double, borderRadius: Double, style: TextStyle, focusOnMount: Bool, placeholder: String, outlineColor: Color, outlineWidth: Double, isTextArea: Bool) {
        self.id = id
        self.parentFrame = parentFrame
        self.renderLayerId = renderLayerId
        self.zIndex = zIndex
        self.transform = transform
        self.size_x = size_x
        self.size_y = size_y
        self.opacity = opacity
        self.text = text
        self.background = background
        self.strokeColor = strokeColor
        self.strokeWidth = strokeWidth
        self.borderRadius = borderRadius
        self.style = style
        self.focusOnMount = focusOnMount
        self.placeholder = placeholder
        self.outlineColor = outlineColor
        self.outlineWidth = outlineWidth
        self.isTextArea = isTextArea
    }

    public static func makeDefault(id: PaxNodeId, parentFrame: PaxNodeId?, renderLayerId: UInt32) -> TextboxElement {
        TextboxElement(id: id, parentFrame: parentFrame, renderLayerId: renderLayerId, zIndex: 0, transform: [1, 0, 0, 1, 0, 0], size_x: 0, size_y: 0, opacity: 1.0, text: "", background: Color(.white), strokeColor: Color(.gray), strokeWidth: 1, borderRadius: 8, style: defaultPaxTextStyle(), focusOnMount: false, placeholder: "", outlineColor: Color(.clear), outlineWidth: 0, isTextArea: false)
    }

    public func applyPatch(_ patch: TextboxUpdatePatch) {
        if let transform = patch.transform { self.transform = transform }
        if let size_x = patch.size_x { self.size_x = size_x }
        if let size_y = patch.size_y { self.size_y = size_y }
        if let opacity = patch.opacity { self.opacity = opacity }
        if let text = patch.text { self.text = text }
        if let background = patch.background { self.background = background }
        if let strokeColor = patch.strokeColor { self.strokeColor = strokeColor }
        if let strokeWidth = patch.strokeWidth { self.strokeWidth = strokeWidth }
        if let borderRadius = patch.borderRadius { self.borderRadius = borderRadius }
        if let style = patch.style { self.style.applyPatch(from: style) }
        if let focusOnMount = patch.focusOnMount { self.focusOnMount = focusOnMount }
        if let placeholder = patch.placeholder { self.placeholder = placeholder }
        if let outlineColor = patch.outlineColor { self.outlineColor = outlineColor }
        if let outlineWidth = patch.outlineWidth { self.outlineWidth = outlineWidth }
        if let isTextArea = patch.isTextArea { self.isTextArea = isTextArea }
    }
}

public class NavigationPatchMessage {
    public var url: String
    public var target: String

    public init(fb: FlxbReference) {
        self.url = fb["url"]?.asString ?? ""
        self.target = fb["target"]?.asString ?? "current"
    }
}

public class SetCursorPatchMessage {
    public var cursor: String

    public init(fb: FlxbReference) {
        self.cursor = fb["cursor"]?.asString ?? "default"
    }
}
