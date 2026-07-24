import XCTest
@testable import Messages
import FlexBuffers

final class PaxSwiftCommonTests: XCTestCase {
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
