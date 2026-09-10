// MICAFP UnifiedShield VIP-ULTRA — iOS BLE Mesh Manager
// Manages Bluetooth LE scanning and advertising for mesh peer discovery.
// Works even when WiFi and cellular are completely blocked.

import Foundation
import CoreBluetooth

/// Service UUID for MICAFP Shield BLE mesh.
let ShieldServiceUUID = CBUUID(string: "A1B2C3D4-E5F6-7890-ABCD-EF1234567890")
/// Characteristic UUID for yggdrasil public key exchange.
let YggdrasilPubKeyCharUUID = CBUUID(string: "A1B2C3D4-E5F6-7890-ABCD-EF1234567891")
/// Characteristic UUID for mesh data exchange.
let MeshDataCharUUID = CBUUID(string: "A1B2C3D4-E5F6-7890-ABCD-EF1234567892")

@objc public class BleMeshManager: NSObject, CBCentralManagerDelegate, CBPeripheralManagerDelegate {

    private var centralManager: CBCentralManager?
    private var peripheralManager: CBPeripheralManager?
    private var discoveredPeers: [CBPeripheral] = []

    @objc public var isScanning: Bool = false
    @objc public var discoveredPeerCount: Int { return discoveredPeers.count }
    @objc public var onPeerDiscovered: ((String) -> Void)?

    // MARK: - Start/Stop
    @objc public func startMesh() {
        centralManager = CBCentralManager(delegate: self, queue: nil)
        peripheralManager = CBPeripheralManager(delegate: self, queue: nil)
    }

    @objc public func stopMesh() {
        centralManager?.stopScan()
        peripheralManager?.stopAdvertising()
        isScanning = false
    }

    // MARK: - CBCentralManagerDelegate
    public func centralManagerDidUpdateState(_ central: CBCentralManager) {
        if central.state == .poweredOn {
            central.scanForPeripherals(withServices: [ShieldServiceUUID], options: [
                CBCentralManagerScanOptionAllowDuplicatesKey: false
            ])
            isScanning = true
        }
    }

    public func centralManager(_ central: CBCentralManager,
                                didDiscover peripheral: CBPeripheral,
                                advertisementData: [String: Any],
                                rssi RSSI: NSNumber) {
        guard !discoveredPeers.contains(peripheral) else { return }
        discoveredPeers.append(peripheral)
        onPeerDiscovered?(peripheral.identifier.uuidString)
    }

    // MARK: - CBPeripheralManagerDelegate
    public func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        if peripheral.state == .poweredOn {
            let service = CBMutableService(type: ShieldServiceUUID, primary: true)
            let yggChar = CBMutableCharacteristic(
                type: YggdrasilPubKeyCharUUID,
                properties: [.read, .notify],
                value: nil,
                permissions: [.readable]
            )
            service.characteristics = [yggChar]
            peripheral.add(service)
            peripheral.startAdvertising([
                CBAdvertisementDataServiceUUIDsKey: [ShieldServiceUUID],
                CBAdvertisementDataLocalNameKey: "Shield"
            ])
        }
    }

    public func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {}
}
