#if os(iOS)
import CoreMotion
import Foundation

public final class PaxMotionSensorBridge {
    public static let shared = PaxMotionSensorBridge()

    private let motionManager = CMMotionManager()
    private let motionQueue = OperationQueue()
    private let metersPerSecondSquaredPerG = 9.80665
    private let degreesPerRadian = 180.0 / Double.pi
    private var isStarted = false

    private init() {
        motionQueue.name = "dev.pax.motion-sensors"
        motionQueue.qualityOfService = .userInteractive
    }

    public func start() {
        guard !isStarted, motionManager.isDeviceMotionAvailable else {
            return
        }

        isStarted = true
        motionManager.deviceMotionUpdateInterval = 1.0 / 60.0
        motionManager.startDeviceMotionUpdates(using: .xArbitraryZVertical, to: motionQueue) { [weak self] motion, _ in
            guard let self, let motion else {
                return
            }

            let attitude = motion.attitude
            let acceleration = motion.userAcceleration
            let gravity = motion.gravity
            let gyroX = attitude.pitch * self.degreesPerRadian
            let gyroY = attitude.roll * self.degreesPerRadian
            let gyroZ = attitude.yaw * self.degreesPerRadian
            let accelX = (acceleration.x + gravity.x) * self.metersPerSecondSquaredPerG
            let accelY = (acceleration.y + gravity.y) * self.metersPerSecondSquaredPerG
            let accelZ = (acceleration.z + gravity.z) * self.metersPerSecondSquaredPerG

            DispatchQueue.main.async {
                dispatchGyro(x: gyroX, y: gyroY, z: gyroZ)
                dispatchAccel(x: accelX, y: accelY, z: accelZ)
            }
        }
    }

    public func stop() {
        guard isStarted else {
            return
        }

        motionManager.stopDeviceMotionUpdates()
        isStarted = false
    }
}
#endif
