import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class StoredSessionCredential {
  const StoredSessionCredential({
    required this.accountId,
    required this.refreshToken,
  });

  final String accountId;
  final String refreshToken;
}

abstract interface class SecureSessionStorage {
  Future<StoredSessionCredential?> readSession();

  Future<void> replaceSession({
    required String accountId,
    required String refreshToken,
  });

  Future<void> clearSession(String accountId);
}

class KeychainKeystoreSessionStorage implements SecureSessionStorage {
  KeychainKeystoreSessionStorage({
    required String environmentNamespace,
    FlutterSecureStorage? storage,
  }) : _environmentNamespace = _validateNamespace(environmentNamespace),
       _storage =
           storage ??
           FlutterSecureStorage(
             aOptions: AndroidOptions(
               storageNamespace:
                   'de.yourtj.mobile.session.$environmentNamespace',
               resetOnError: false,
               migrateOnAlgorithmChange: true,
               migrateWithBackup: false,
             ),
             iOptions: IOSOptions(
               accountName: 'de.yourtj.mobile.session.$environmentNamespace',
               accessibility: KeychainAccessibility.first_unlock_this_device,
               synchronizable: false,
             ),
           );

  static const int _schemaVersion = 2;
  static const int _maxRefreshTokenLength = 256;
  static final RegExp _safeNamespace = RegExp(r'^[A-Za-z0-9_-]{1,200}$');
  static final RegExp _accountId = RegExp(r'^[A-Za-z0-9-]{1,128}$');
  final String _environmentNamespace;
  final FlutterSecureStorage _storage;

  String get _sessionKey =>
      'session.$_environmentNamespace.v$_schemaVersion.active-session';

  String get _legacyActiveAccountKey =>
      'session.$_environmentNamespace.v1.active-account';

  String _legacyRefreshKey(String accountId) {
    _validateAccountId(accountId);
    return 'session.$_environmentNamespace.v1.refresh.$accountId';
  }

  static String _validateNamespace(String value) {
    if (!_safeNamespace.hasMatch(value)) {
      throw const FormatException('环境存储命名空间无效');
    }
    return value;
  }

  static void _validateAccountId(String value) {
    if (!_accountId.hasMatch(value)) {
      throw const FormatException('账号标识无效');
    }
  }

  static void _validateRefreshToken(String value) {
    if (value.isEmpty || value.length > _maxRefreshTokenLength) {
      throw const FormatException('刷新凭据无效');
    }
  }

  @override
  Future<StoredSessionCredential?> readSession() async {
    final String? encoded = await _storage.read(key: _sessionKey);
    if (encoded != null) {
      final StoredSessionCredential session = _decodeSession(encoded);
      await _clearLegacyBestEffort();
      return session;
    }

    final String? legacyAccountId = await _readLegacyAccountId();
    if (legacyAccountId == null) {
      return null;
    }
    final String? legacyRefreshToken = await _storage.read(
      key: _legacyRefreshKey(legacyAccountId),
    );
    if (legacyRefreshToken == null || legacyRefreshToken.isEmpty) {
      await _clearLegacyStrict(expectedAccountId: legacyAccountId);
      return null;
    }
    _validateRefreshToken(legacyRefreshToken);

    final StoredSessionCredential session = StoredSessionCredential(
      accountId: legacyAccountId,
      refreshToken: legacyRefreshToken,
    );
    await _storage.write(key: _sessionKey, value: _encodeSession(session));
    await _clearLegacyBestEffort();
    return session;
  }

  @override
  Future<void> replaceSession({
    required String accountId,
    required String refreshToken,
  }) async {
    _validateAccountId(accountId);
    _validateRefreshToken(refreshToken);
    final StoredSessionCredential session = StoredSessionCredential(
      accountId: accountId,
      refreshToken: refreshToken,
    );
    await _storage.write(key: _sessionKey, value: _encodeSession(session));
    await _clearLegacyBestEffort();
  }

  @override
  Future<void> clearSession(String accountId) async {
    _validateAccountId(accountId);
    final String? encoded = await _storage.read(key: _sessionKey);
    final StoredSessionCredential? current = encoded == null
        ? null
        : _decodeSession(encoded);

    if (current != null && current.accountId != accountId) {
      await _clearLegacyStrict(expectedAccountId: accountId);
      return;
    }

    await _clearLegacyStrict(
      expectedAccountId: current == null ? accountId : null,
    );
    if (current != null) {
      await _storage.delete(key: _sessionKey);
    }
  }

  String _encodeSession(StoredSessionCredential session) {
    return jsonEncode(<String, Object>{
      'schemaVersion': _schemaVersion,
      'accountId': session.accountId,
      'refreshToken': session.refreshToken,
    });
  }

  StoredSessionCredential _decodeSession(String encoded) {
    try {
      final Object? decoded = jsonDecode(encoded);
      if (decoded is! Map ||
          decoded['schemaVersion'] != _schemaVersion ||
          decoded['accountId'] is! String ||
          decoded['refreshToken'] is! String) {
        throw const FormatException('会话凭据格式无效');
      }
      final String accountId = decoded['accountId']! as String;
      final String refreshToken = decoded['refreshToken']! as String;
      _validateAccountId(accountId);
      _validateRefreshToken(refreshToken);
      return StoredSessionCredential(
        accountId: accountId,
        refreshToken: refreshToken,
      );
    } on FormatException {
      rethrow;
    } on Object {
      throw const FormatException('会话凭据无法解码');
    }
  }

  Future<String?> _readLegacyAccountId() async {
    final String? accountId = await _storage.read(key: _legacyActiveAccountKey);
    if (accountId != null) {
      _validateAccountId(accountId);
    }
    return accountId;
  }

  Future<void> _clearLegacyBestEffort() async {
    try {
      await _clearLegacyStrict();
    } on Object {
      // A committed v2 record remains authoritative. Logout retries legacy cleanup
      // before deleting it, so a stale v1 token cannot silently resurrect later.
    }
  }

  Future<void> _clearLegacyStrict({String? expectedAccountId}) async {
    final String? legacyAccountId = await _readLegacyAccountId();
    if (legacyAccountId == null ||
        (expectedAccountId != null && legacyAccountId != expectedAccountId)) {
      return;
    }
    await _storage.delete(key: _legacyRefreshKey(legacyAccountId));
    await _storage.delete(key: _legacyActiveAccountKey);
  }
}
