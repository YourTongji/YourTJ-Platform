import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

abstract interface class WalletSeedStore {
  Future<List<int>?> read(String accountId);

  Future<void> write(String accountId, List<int> seed);

  Future<void> delete(String accountId);
}

class KeychainKeystoreWalletSeedStore implements WalletSeedStore {
  KeychainKeystoreWalletSeedStore({
    required String environmentNamespace,
    FlutterSecureStorage? storage,
  }) : _environmentNamespace = _validateNamespace(environmentNamespace),
       _storage =
           storage ??
           FlutterSecureStorage(
             aOptions: AndroidOptions(
               storageNamespace:
                   'de.yourtj.mobile.wallet.$environmentNamespace',
               resetOnError: false,
               migrateOnAlgorithmChange: true,
               migrateWithBackup: false,
             ),
             iOptions: IOSOptions(
               accountName: 'de.yourtj.mobile.wallet.$environmentNamespace',
               accessibility: KeychainAccessibility.passcode,
               synchronizable: false,
             ),
           );

  static const int _keyVersion = 1;
  static final RegExp _safeNamespace = RegExp(r'^[A-Za-z0-9_-]{1,200}$');
  static final RegExp _accountId = RegExp(r'^[A-Za-z0-9-]{1,128}$');
  final String _environmentNamespace;
  final FlutterSecureStorage _storage;

  String _key(String accountId) {
    if (!_accountId.hasMatch(accountId)) {
      throw const FormatException('账号标识无效');
    }
    return 'wallet.$_environmentNamespace.seed.v$_keyVersion.$accountId';
  }

  static String _validateNamespace(String value) {
    if (!_safeNamespace.hasMatch(value)) {
      throw const FormatException('环境存储命名空间无效');
    }
    return value;
  }

  @override
  Future<List<int>?> read(String accountId) async {
    final String? encoded = await _storage.read(key: _key(accountId));
    if (encoded == null) {
      return null;
    }
    try {
      final List<int> seed = base64Decode(encoded);
      if (seed.length != 32) {
        throw const FormatException('钱包密钥长度无效');
      }
      return seed;
    } on FormatException {
      throw const FormatException('钱包密钥无法解码');
    }
  }

  @override
  Future<void> write(String accountId, List<int> seed) async {
    if (seed.length != 32) {
      throw const FormatException('钱包密钥长度无效');
    }
    await _storage.write(key: _key(accountId), value: base64Encode(seed));
  }

  @override
  Future<void> delete(String accountId) =>
      _storage.delete(key: _key(accountId));
}
