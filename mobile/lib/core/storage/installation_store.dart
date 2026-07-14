import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:uuid/uuid.dart';

abstract interface class InstallationStore {
  Future<String> readOrCreateId();
}

abstract interface class InstallationPreferences {
  Future<String?> getString(String key);

  Future<void> setString(String key, String value);
}

abstract interface class IosInstallationIdChannel {
  Future<String> readOrCreateId(String environmentNamespace);
}

final class InstallationStoreException implements Exception {
  const InstallationStoreException();

  @override
  String toString() => 'InstallationStoreException: identifier unavailable';
}

InstallationStore createInstallationStore({
  required String environmentNamespace,
  bool? isWeb,
  TargetPlatform? targetPlatform,
  InstallationPreferences? preferences,
  IosInstallationIdChannel? iosChannel,
  String Function()? uuidV4,
}) {
  final bool resolvedIsWeb = isWeb ?? kIsWeb;
  final TargetPlatform resolvedTargetPlatform =
      targetPlatform ?? defaultTargetPlatform;
  if (!resolvedIsWeb && resolvedTargetPlatform == TargetPlatform.iOS) {
    return IosNoBackupInstallationStore(
      environmentNamespace: environmentNamespace,
      channel: iosChannel,
    );
  }
  return SharedPreferencesInstallationStore(
    environmentNamespace: environmentNamespace,
    preferences: preferences,
    uuidV4: uuidV4,
  );
}

class SharedPreferencesInstallationStore implements InstallationStore {
  SharedPreferencesInstallationStore({
    required String environmentNamespace,
    InstallationPreferences? preferences,
    String Function()? uuidV4,
  }) : _installationIdKey = _keyFor(environmentNamespace),
       _preferences =
           preferences ?? _SharedPreferencesInstallationPreferences(),
       _uuidV4 = uuidV4 ?? const Uuid().v4;

  final InstallationPreferences _preferences;
  final String Function() _uuidV4;
  final String _installationIdKey;

  static String _keyFor(String environmentNamespace) {
    _validateEnvironmentNamespace(environmentNamespace);
    return 'yourtj.installation.v1.$environmentNamespace';
  }

  @override
  Future<String> readOrCreateId() async {
    final String? existing = await _preferences.getString(_installationIdKey);
    if (existing != null && _isUuidV4(existing)) {
      return existing.toLowerCase();
    }
    final String installationId = _uuidV4();
    if (!_isUuidV4(installationId)) {
      throw const InstallationStoreException();
    }
    final String normalizedInstallationId = installationId.toLowerCase();
    await _preferences.setString(_installationIdKey, normalizedInstallationId);
    return normalizedInstallationId;
  }
}

class IosNoBackupInstallationStore implements InstallationStore {
  IosNoBackupInstallationStore({
    required this.environmentNamespace,
    IosInstallationIdChannel? channel,
  }) : _channel = channel ?? MethodChannelIosInstallationIdChannel() {
    _validateEnvironmentNamespace(environmentNamespace);
  }

  final String environmentNamespace;
  final IosInstallationIdChannel _channel;

  @override
  Future<String> readOrCreateId() async {
    final String installationId = await _channel.readOrCreateId(
      environmentNamespace,
    );
    if (!_isUuidV4(installationId)) {
      throw const InstallationStoreException();
    }
    return installationId.toLowerCase();
  }
}

class MethodChannelIosInstallationIdChannel
    implements IosInstallationIdChannel {
  MethodChannelIosInstallationIdChannel({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel(_channelName);

  static const String _channelName = 'de.yourtj.mobile/installation';
  static const String _readOrCreateMethod = 'readOrCreateInstallationId';

  final MethodChannel _channel;

  @override
  Future<String> readOrCreateId(String environmentNamespace) async {
    _validateEnvironmentNamespace(environmentNamespace);
    try {
      final String? installationId = await _channel.invokeMethod<String>(
        _readOrCreateMethod,
        <String, String>{'environmentNamespace': environmentNamespace},
      );
      if (installationId == null || !_isUuidV4(installationId)) {
        throw const InstallationStoreException();
      }
      return installationId.toLowerCase();
    } on MissingPluginException {
      throw const InstallationStoreException();
    } on PlatformException {
      throw const InstallationStoreException();
    }
  }
}

final RegExp _safeNamespace = RegExp(r'^[A-Za-z0-9_-]{1,200}$');
final RegExp _uuidV4 = RegExp(
  r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
  caseSensitive: false,
);

void _validateEnvironmentNamespace(String environmentNamespace) {
  final RegExpMatch? match = _safeNamespace.firstMatch(environmentNamespace);
  if (match == null || match.end != environmentNamespace.length) {
    throw ArgumentError.value(
      environmentNamespace,
      'environmentNamespace',
      'must contain only ASCII letters, digits, underscores, or hyphens',
    );
  }
}

bool _isUuidV4(String value) {
  final RegExpMatch? match = _uuidV4.firstMatch(value);
  return match != null && match.end == value.length;
}

class _SharedPreferencesInstallationPreferences
    implements InstallationPreferences {
  _SharedPreferencesInstallationPreferences()
    : _preferences = SharedPreferencesAsync();

  final SharedPreferencesAsync _preferences;

  @override
  Future<String?> getString(String key) => _preferences.getString(key);

  @override
  Future<void> setString(String key, String value) =>
      _preferences.setString(key, value);
}
