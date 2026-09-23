import XCTest
@testable import Messages
import FlexBuffers

final class PaxSwiftCommonTests: XCTestCase {
    func testSafeAreaInsetsInterruptKeepsEveryEdge() throws {
        var sentData: Data?
        let previousSender = NativeInterruptDispatcher.shared.sendData
        NativeInterruptDispatcher.shared.sendData = { sentData = $0 }
        defer { NativeInterruptDispatcher.shared.sendData = previousSender }
        dispatchSafeAreaInsets(top: 62, right: 0, bottom: 34, left: 0)
        var root = try XCTUnwrap(FlexBuffer.decode(data: try XCTUnwrap(sentData)))
        XCTAssertEqual(root["SafeAreaInsets"]?["top"]?.asDouble, 62)
        XCTAssertEqual(root["SafeAreaInsets"]?["bottom"]?.asDouble, 34)
        dispatchSafeAreaInsets(top: 0, right: 62, bottom: 21, left: 62)
        root = try XCTUnwrap(FlexBuffer.decode(data: try XCTUnwrap(sentData)))
        XCTAssertEqual(root["SafeAreaInsets"]?["top"]?.asDouble, 0)
        XCTAssertEqual(root["SafeAreaInsets"]?["right"]?.asDouble, 62)
        XCTAssertEqual(root["SafeAreaInsets"]?["left"]?.asDouble, 62)
        XCTAssertEqual(root["SafeAreaInsets"]?["bottom"]?.asDouble, 21)
    }

    func testFamilyOnlyFontPatchPreservesFamilyAndTraits() throws {
        let payload = try FlexBufferBuilder.fromJSON(
            #"{"Web":{"family":"Courier","url":"","style":"Italic","weight":"Bold"}}"#
        )
        let font = PaxFont.makeDefault()
        font.applyPatch(fb: try XCTUnwrap(payload.root))
        guard case .system(let system) = font.type else {
            return XCTFail("A family-only font should resolve from installed fonts")
        }
        XCTAssertEqual(system.family, "Courier")
        XCTAssertEqual(system.style, .italic)
        XCTAssertEqual(system.weight, .bold)
    }

    func testFontPatchWithSourceRetainsWebFontLoading() throws {
        let payload = try FlexBufferBuilder.fromJSON(
            #"{"Web":{"family":"Example","url":"https://example.com/font.ttf","style":"Normal","weight":"Medium"}}"#
        )
        let font = PaxFont.makeDefault()
        font.applyPatch(fb: try XCTUnwrap(payload.root))
        guard case .web(let web) = font.type else {
            return XCTFail("A font with a source should retain its loader")
        }
        XCTAssertEqual(web.family, "Example")
        XCTAssertEqual(web.url.absoluteString, "https://example.com/font.ttf")
        XCTAssertEqual(web.weight, .medium)
    }

    func testFontFamilyMatchingRejectsPartialNameCollisions() {
        XCTAssertTrue(PaxFont.fontFamilyNamesMatch(
            candidate: "Times New-Roman",
            requested: "times new roman"
        ))
        XCTAssertFalse(PaxFont.fontFamilyNamesMatch(candidate: "SignPainter", requested: "Inter"))
        XCTAssertFalse(PaxFont.fontFamilyNamesMatch(candidate: "Roboto Slab", requested: "Roboto"))
    }

    func testUnsuffixedRegularFacesCompeteWithNamedWeights() {
        for (family, regular, bold) in [
            ("Courier New", "CourierNewPSMT", "CourierNewPS-BoldMT"),
            ("Arial", "ArialMT", "Arial-BoldMT"),
        ] {
            func score(_ name: String, _ weight: FontWeight) -> Int {
                PaxFont.candidateScore(
                    candidateName: name, requestedFamily: family,
                    style: .normal, weight: weight, isFamilyName: false
                )
            }
            XCTAssertGreaterThan(score(regular, .normal), score(bold, .normal))
            XCTAssertGreaterThan(score(regular, .medium), score(bold, .medium))
            XCTAssertGreaterThan(score(bold, .bold), score(regular, .bold))
        }
    }

    func testFontWeightSelectionUsesCSSOrderingForEveryFamily() {
        XCTAssertLessThan(
            PaxFont.weightSelectionRank(candidate: .bold, requested: .semiBold),
            PaxFont.weightSelectionRank(candidate: .medium, requested: .semiBold)
        )
        XCTAssertLessThan(
            PaxFont.weightSelectionRank(candidate: .medium, requested: .normal),
            PaxFont.weightSelectionRank(candidate: .light, requested: .normal)
        )
        XCTAssertLessThan(
            PaxFont.weightSelectionRank(candidate: .extraLight, requested: .light),
            PaxFont.weightSelectionRank(candidate: .normal, requested: .light)
        )
        XCTAssertEqual(PaxFont.weightSelectionRank(candidate: .black, requested: .black), 0)
    }

    func testTouchCancelDispatchesDistinctInterrupt() throws {
        var sentData: Data?
        let previousSender = NativeInterruptDispatcher.shared.sendData
        NativeInterruptDispatcher.shared.sendData = { sentData = $0 }
        defer { NativeInterruptDispatcher.shared.sendData = previousSender }

        dispatchTouchCancel(touches: [
            TouchInterruptMessage(
                x: 12.5,
                y: 34.5,
                identifier: 7,
                deltaX: 2.0,
                deltaY: -3.0
            ),
        ])

        let data = try XCTUnwrap(sentData)
        let root = try XCTUnwrap(FlexBuffer.decode(data: data))
        let touchCancel = try XCTUnwrap(root["TouchCancel"])
        let touches = try XCTUnwrap(touchCancel["touches"]?.asVector)
        let touchIterator = touches.makeIterator()
        let touch = try XCTUnwrap(touchIterator.next())
        XCTAssertEqual(touch["x"]?.asDouble, 12.5)
        XCTAssertEqual(touch["y"]?.asDouble, 34.5)
        XCTAssertEqual(touch["identifier"]?.asInt, 7)
        XCTAssertEqual(touch["delta_x"]?.asDouble, 2.0)
        XCTAssertEqual(touch["delta_y"]?.asDouble, -3.0)
    }

    func testVirtualRouteNavigationDispatchesRouteChangeInterrupt() throws {
        let coordinator = PaxVirtualRouteCoordinator()
        var sentData: Data?
        let previousSender = NativeInterruptDispatcher.shared.sendData
        NativeInterruptDispatcher.shared.sendData = { sentData = $0 }
        defer { NativeInterruptDispatcher.shared.sendData = previousSender }

        XCTAssertTrue(coordinator.navigate(to: "/guide/topic/router?view=api&tag=a&tag=b#bindings"))

        let data = try XCTUnwrap(sentData)
        let root = try XCTUnwrap(FlexBuffer.decode(data: data))
        let routeChange = try XCTUnwrap(root["RouteChange"])
        XCTAssertEqual(
            routeChange["path_segments"]?.asVector?.makeIterator().compactMap { $0.asString },
            ["guide", "topic", "router"]
        )
        XCTAssertEqual(
            routeChange["query"]?["view"]?.asVector?.makeIterator().compactMap { $0.asString },
            ["api"]
        )
        XCTAssertEqual(
            routeChange["query"]?["tag"]?.asVector?.makeIterator().compactMap { $0.asString },
            ["a", "b"]
        )
        XCTAssertEqual(routeChange["fragment"]?.asString, "bindings")
    }

    func testVirtualRouteResolutionUsesCurrentLocationForRelativeDestinations() throws {
        let coordinator = PaxVirtualRouteCoordinator()

        XCTAssertTrue(coordinator.navigate(to: "/teams/42/members?lane=alpha"))

        let queryOnly = try XCTUnwrap(coordinator.resolve(destination: "?lane=beta#inspect"))
        XCTAssertEqual(queryOnly.pathSegments, ["teams", "42", "members"])
        XCTAssertEqual(queryOnly.query["lane"], ["beta"])
        XCTAssertEqual(queryOnly.fragment, "inspect")

        let sibling = try XCTUnwrap(coordinator.resolve(destination: "./settings"))
        XCTAssertEqual(sibling.pathSegments, ["teams", "42", "settings"])

        XCTAssertNil(coordinator.resolve(destination: "https://docs.pax.dev"))
    }
}
