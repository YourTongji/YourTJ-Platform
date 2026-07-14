import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  private var installationChannel: FlutterMethodChannel?
  private var accountExportChannel: FlutterMethodChannel?
  private let accountExportCoordinator = AccountExportCoordinator()
  private let installationStoreQueue = DispatchQueue(
    label: "de.yourtj.mobile.installation-store"
  )

  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    accountExportCoordinator.cleanOrphansAtLaunch()
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)

    let channel = FlutterMethodChannel(
      name: "de.yourtj.mobile/installation",
      binaryMessenger: engineBridge.applicationRegistrar.messenger()
    )
    installationChannel = channel
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else {
        result(
          FlutterError(
            code: "INSTALLATION_STORE_UNAVAILABLE",
            message: "Installation identifier is unavailable.",
            details: nil
          )
        )
        return
      }
      self.handleInstallationMethod(call, result: result)
    }

    let exportChannel = FlutterMethodChannel(
      name: "de.yourtj.mobile/account-export",
      binaryMessenger: engineBridge.applicationRegistrar.messenger()
    )
    accountExportChannel = exportChannel
    exportChannel.setMethodCallHandler { [weak self] call, result in
      guard let self else {
        result(
          FlutterError(
            code: "EXPORT_UNAVAILABLE",
            message: "Account export is unavailable.",
            details: nil
          )
        )
        return
      }
      self.accountExportCoordinator.handle(
        call,
        presenter: self.topViewController(from: self.window?.rootViewController),
        result: result
      )
    }
  }

  private func handleInstallationMethod(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard call.method == "readOrCreateInstallationId" else {
      result(FlutterMethodNotImplemented)
      return
    }
    guard
      let arguments = call.arguments as? [String: Any],
      let environmentNamespace = arguments["environmentNamespace"] as? String
    else {
      result(
        FlutterError(
          code: "INVALID_ARGUMENT",
          message: "A valid environment namespace is required.",
          details: nil
        )
      )
      return
    }

    installationStoreQueue.async { [weak self] in
      guard let self else {
        DispatchQueue.main.async {
          result(
            FlutterError(
              code: "INSTALLATION_STORE_UNAVAILABLE",
              message: "Installation identifier is unavailable.",
              details: nil
            )
          )
        }
        return
      }
      do {
        let installationId = try self.readOrCreateInstallationId(
          environmentNamespace: environmentNamespace
        )
        DispatchQueue.main.async {
          result(installationId)
        }
      } catch InstallationIdStoreError.invalidNamespace {
        DispatchQueue.main.async {
          result(
            FlutterError(
              code: "INVALID_ARGUMENT",
              message: "A valid environment namespace is required.",
              details: nil
            )
          )
        }
      } catch {
        DispatchQueue.main.async {
          result(
            FlutterError(
              code: "INSTALLATION_STORE_UNAVAILABLE",
              message: "Installation identifier is unavailable.",
              details: nil
            )
          )
        }
      }
    }
  }

  private func readOrCreateInstallationId(environmentNamespace: String) throws -> String {
    guard Self.isSafeNamespace(environmentNamespace) else {
      throw InstallationIdStoreError.invalidNamespace
    }

    let fileManager = FileManager.default
    guard
      let applicationSupport = fileManager.urls(
        for: .applicationSupportDirectory,
        in: .userDomainMask
      ).first
    else {
      throw InstallationIdStoreError.applicationSupportUnavailable
    }
    let directory = applicationSupport
      .appendingPathComponent("YourTJ", isDirectory: true)
      .appendingPathComponent("Installation", isDirectory: true)
      .appendingPathComponent("v1", isDirectory: true)
    try fileManager.createDirectory(
      at: directory,
      withIntermediateDirectories: true,
      attributes: nil
    )
    try Self.excludeFromBackup(directory)

    let file = directory.appendingPathComponent(
      "\(environmentNamespace).id",
      isDirectory: false
    )
    if fileManager.fileExists(atPath: file.path) {
      let data = try Data(contentsOf: file)
      guard
        let storedValue = String(data: data, encoding: .utf8),
        Self.isUuidV4(storedValue)
      else {
        throw InstallationIdStoreError.invalidStoredIdentifier
      }
      try Self.excludeFromBackup(file)
      return storedValue.lowercased()
    }

    let installationId = UUID().uuidString.lowercased()
    guard Self.isUuidV4(installationId) else {
      throw InstallationIdStoreError.invalidGeneratedIdentifier
    }
    try Data(installationId.utf8).write(to: file, options: .atomic)
    try Self.excludeFromBackup(file)
    return installationId
  }

  private static func excludeFromBackup(_ url: URL) throws {
    var mutableUrl = url
    var resourceValues = URLResourceValues()
    resourceValues.isExcludedFromBackup = true
    try mutableUrl.setResourceValues(resourceValues)
  }

  private static func isSafeNamespace(_ value: String) -> Bool {
    guard !value.isEmpty, value.utf8.count <= 200 else {
      return false
    }
    return value.unicodeScalars.allSatisfy { scalar in
      let codePoint = scalar.value
      return (48...57).contains(codePoint)
        || (65...90).contains(codePoint)
        || (97...122).contains(codePoint)
        || codePoint == 45
        || codePoint == 95
    }
  }

  private static func isUuidV4(_ value: String) -> Bool {
    let pattern = "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$"
    guard let expression = try? NSRegularExpression(pattern: pattern) else {
      return false
    }
    let range = NSRange(value.startIndex..<value.endIndex, in: value)
    return expression.firstMatch(in: value, range: range)?.range == range
  }

  private func topViewController(from root: UIViewController?) -> UIViewController? {
    if let presented = root?.presentedViewController {
      return topViewController(from: presented)
    }
    if let navigation = root as? UINavigationController {
      return topViewController(from: navigation.visibleViewController)
    }
    if let tabs = root as? UITabBarController {
      return topViewController(from: tabs.selectedViewController)
    }
    return root
  }
}

private enum InstallationIdStoreError: Error {
  case applicationSupportUnavailable
  case invalidGeneratedIdentifier
  case invalidNamespace
  case invalidStoredIdentifier
}

private final class AccountExportCoordinator: NSObject,
  UIDocumentPickerDelegate,
  UIAdaptivePresentationControllerDelegate
{
  private static let saveMethod = "saveAccountExport"
  private static let cancelMethod = "cancelAccountExport"
  private static let fileName = "yourtj-account-export.json"
  private static let maxPayloadBytes = 16 * 1024 * 1024
  private static let directoryName = "YourTJAccountExports"

  private var pending: PendingExport?
  private weak var picker: UIDocumentPickerViewController?

  func cleanOrphansAtLaunch() {
    try? removeExportRoot()
  }

  func handle(
    _ call: FlutterMethodCall,
    presenter: UIViewController?,
    result: @escaping FlutterResult
  ) {
    switch call.method {
    case Self.saveMethod:
      save(call, presenter: presenter, result: result)
    case Self.cancelMethod:
      cancel(result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func save(
    _ call: FlutterMethodCall,
    presenter: UIViewController?,
    result: @escaping FlutterResult
  ) {
    guard pending == nil else {
      result(
        FlutterError(
          code: "EXPORT_BUSY",
          message: "An account export is already pending.",
          details: nil
        )
      )
      return
    }
    guard
      let arguments = call.arguments as? [String: Any],
      arguments["fileName"] as? String == Self.fileName,
      let typedData = arguments["bytes"] as? FlutterStandardTypedData,
      !typedData.data.isEmpty,
      typedData.data.count <= Self.maxPayloadBytes
    else {
      result(
        FlutterError(
          code: "INVALID_ARGUMENT",
          message: "The account export payload is invalid.",
          details: nil
        )
      )
      return
    }
    guard let presenter, presenter.viewIfLoaded?.window != nil else {
      result(
        FlutterError(
          code: "EXPORT_UNAVAILABLE",
          message: "No document picker presenter is available.",
          details: nil
        )
      )
      return
    }

    let directory: URL
    let file: URL
    do {
      try removeExportRoot()
      let root = exportRoot()
      try createProtectedDirectory(root)
      directory = root.appendingPathComponent(UUID().uuidString, isDirectory: true)
      try createProtectedDirectory(directory)
      file = directory.appendingPathComponent(Self.fileName, isDirectory: false)
      try typedData.data.write(to: file, options: [.atomic, .completeFileProtection])
      try FileManager.default.setAttributes(
        [
          .posixPermissions: NSNumber(value: 0o600),
          .protectionKey: FileProtectionType.complete,
        ],
        ofItemAtPath: file.path
      )
    } catch {
      if (try? removeExportRoot()) == nil {
        result(cleanupError())
      } else {
        result(
          FlutterError(
            code: "EXPORT_WRITE_FAILED",
            message: "The protected export file could not be prepared.",
            details: nil
          )
        )
      }
      return
    }

    let documentPicker: UIDocumentPickerViewController
    if #available(iOS 14.0, *) {
      documentPicker = UIDocumentPickerViewController(forExporting: [file], asCopy: true)
    } else {
      documentPicker = UIDocumentPickerViewController(url: file, in: .exportToService)
    }
    documentPicker.delegate = self
    documentPicker.shouldShowFileExtensions = true
    pending = PendingExport(directory: directory, result: result)
    picker = documentPicker
    presenter.present(documentPicker, animated: true) { [weak self, weak documentPicker] in
      documentPicker?.presentationController?.delegate = self
    }
  }

  private func cancel(result: @escaping FlutterResult) {
    guard let pending else {
      result(false)
      return
    }
    picker?.dismiss(animated: true)
    finish(pending, status: "cancelled")
    result(true)
  }

  func documentPicker(
    _ controller: UIDocumentPickerViewController,
    didPickDocumentsAt urls: [URL]
  ) {
    guard let pending else {
      return
    }
    if urls.isEmpty {
      finish(
        pending,
        error: FlutterError(
          code: "EXPORT_WRITE_FAILED",
          message: "The system did not confirm an exported file.",
          details: nil
        )
      )
    } else {
      finish(pending, status: "saved")
    }
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    guard let pending else {
      return
    }
    finish(pending, status: "cancelled")
  }

  func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
    guard let pending else {
      return
    }
    finish(pending, status: "cancelled")
  }

  private func finish(_ pending: PendingExport, status: String) {
    guard self.pending === pending else {
      return
    }
    self.pending = nil
    picker = nil
    do {
      try removePreparedExport(pending.directory)
      pending.result(status)
    } catch {
      pending.result(cleanupError())
    }
  }

  private func finish(_ pending: PendingExport, error: FlutterError) {
    guard self.pending === pending else {
      return
    }
    self.pending = nil
    picker = nil
    do {
      try removePreparedExport(pending.directory)
      pending.result(error)
    } catch {
      pending.result(cleanupError())
    }
  }

  private func removePreparedExport(_ directory: URL) throws {
    let fileManager = FileManager.default
    if fileManager.fileExists(atPath: directory.path) {
      try fileManager.removeItem(at: directory)
    }
    let root = exportRoot()
    if fileManager.fileExists(atPath: root.path),
      try fileManager.contentsOfDirectory(atPath: root.path).isEmpty
    {
      try fileManager.removeItem(at: root)
    }
  }

  private func removeExportRoot() throws {
    let root = exportRoot()
    if FileManager.default.fileExists(atPath: root.path) {
      try FileManager.default.removeItem(at: root)
    }
  }

  private func exportRoot() -> URL {
    return FileManager.default.temporaryDirectory.appendingPathComponent(
      Self.directoryName,
      isDirectory: true
    )
  }

  private func createProtectedDirectory(_ directory: URL) throws {
    try FileManager.default.createDirectory(
      at: directory,
      withIntermediateDirectories: true,
      attributes: [
        .posixPermissions: NSNumber(value: 0o700),
        .protectionKey: FileProtectionType.complete,
      ]
    )
    var protectedDirectory = directory
    var resourceValues = URLResourceValues()
    resourceValues.isExcludedFromBackup = true
    try protectedDirectory.setResourceValues(resourceValues)
  }

  private func cleanupError() -> FlutterError {
    return FlutterError(
      code: "EXPORT_CLEANUP_FAILED",
      message: "The protected export staging file could not be removed.",
      details: nil
    )
  }

  private final class PendingExport {
    init(directory: URL, result: @escaping FlutterResult) {
      self.directory = directory
      self.result = result
    }

    let directory: URL
    let result: FlutterResult
  }
}
