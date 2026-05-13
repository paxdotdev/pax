import XCTest
@testable import Messages
import FlexBuffers

final class PaxSwiftCommonTests: XCTestCase {
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
