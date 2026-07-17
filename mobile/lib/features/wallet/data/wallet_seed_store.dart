import 'dart:async';
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

abstract interface class WalletSeedStore {
  Future<List<int>?> read(String accountId);

  Future<List<int>> writeIfAbsent(String accountId, List<int> seed);

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
  Future<void> _mutationTail = Future<void>.value();

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
  Future<List<int>?> read(String accountId) {
    return _serialize(() async {
      final String? encoded = await _storage.read(key: _key(accountId));
      return encoded == null ? null : _decodeSeed(encoded);
    });
  }

  @override
  Future<List<int>> writeIfAbsent(String accountId, List<int> seed) {
    if (seed.length != 32) {
      throw const FormatException('钱包密钥长度无效');
    }
    return _serialize(() async {
      final String key = _key(accountId);
      final String? existing = await _storage.read(key: key);
      if (existing != null) {
        return _decodeSeed(existing);
      }
      await _storage.write(key: key, value: base64Encode(seed));
      return List<int>.from(seed);
    });
  }

  @override
  Future<void> delete(String accountId) {
    return _serialize(() => _storage.delete(key: _key(accountId)));
  }

  List<int> _decodeSeed(String encoded) {
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

  Future<T> _serialize<T>(Future<T> Function() action) async {
    final Completer<void> completion = Completer<void>();
    final Future<void> previous = _mutationTail;
    _mutationTail = completion.future;
    await previous;
    try {
      return await action();
    } finally {
      completion.complete();
    }
  }
}
