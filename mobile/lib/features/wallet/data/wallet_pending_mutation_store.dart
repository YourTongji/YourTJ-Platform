import 'dart:async';
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class WalletPendingMutation {
  WalletPendingMutation({
    required this.operationKey,
    required this.intentId,
    required this.expiresAt,
    required this.action,
  }) {
    if (!_operationKey.hasMatch(operationKey) ||
        !_intentId.hasMatch(intentId) ||
        expiresAt <= 0 ||
        expiresAt > _maxSafeInteger ||
        !_action.hasMatch(action)) {
      throw const FormatException('待核验积分操作格式无效');
    }
  }

  static const int _maxSafeInteger = 9007199254740991;
  static const Set<String> _fields = <String>{
    'operationKey',
    'intentId',
    'expiresAt',
    'action',
  };
  static final RegExp _operationKey = RegExp(
    r'^sha256:(?:[0-9a-f]{64}|[A-Za-z0-9_-]{43})$',
  );
  static final RegExp _intentId = RegExp(
    r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$',
  );
  static final RegExp _action = RegExp(r'^[a-z][a-z0-9_.]{0,79}$');

  final String operationKey;
  final String intentId;
  final int expiresAt;
  final String action;

  Map<String, Object> toJson() => <String, Object>{
    'operationKey': operationKey,
    'intentId': intentId,
    'expiresAt': expiresAt,
    'action': action,
  };

  factory WalletPendingMutation.fromJson(Map<String, Object?> json) {
    if (!_hasExactFields(json, _fields)) {
      throw const FormatException('待核验积分操作字段无效');
    }
    final Object? operationKey = json['operationKey'];
    final Object? intentId = json['intentId'];
    final Object? expiresAt = json['expiresAt'];
    final Object? action = json['action'];
    if (operationKey is! String ||
        intentId is! String ||
        expiresAt is! int ||
        action is! String) {
      throw const FormatException('待核验积分操作格式无效');
    }
    return WalletPendingMutation(
      operationKey: operationKey,
      intentId: intentId,
      expiresAt: expiresAt,
      action: action,
    );
  }

  static bool _hasExactFields(Map<String, Object?> json, Set<String> expected) {
    return json.length == expected.length && json.keys.every(expected.contains);
  }
}

abstract interface class WalletPendingMutationStore {
  Future<List<WalletPendingMutation>> list(String accountId);

  Future<WalletPendingMutation?> read(String accountId, String operationKey);

  Future<void> write(String accountId, WalletPendingMutation mutation);

  Future<bool> delete(String accountId, WalletPendingMutation expectedMutation);
}

class KeychainKeystoreWalletPendingMutationStore
    implements WalletPendingMutationStore {
  KeychainKeystoreWalletPendingMutationStore({
    required String environmentNamespace,
    FlutterSecureStorage? storage,
  }) : _environmentNamespace = _validateNamespace(environmentNamespace),
       _storage =
           storage ??
           FlutterSecureStorage(
             aOptions: AndroidOptions(
               storageNamespace:
                   'de.yourtj.mobile.wallet.pending.$environmentNamespace',
               resetOnError: false,
               migrateOnAlgorithmChange: true,
               migrateWithBackup: false,
             ),
             iOptions: IOSOptions(
               accountName:
                   'de.yourtj.mobile.wallet.pending.$environmentNamespace',
               accessibility: KeychainAccessibility.unlocked_this_device,
               synchronizable: false,
             ),
           );

  static const int _schemaVersion = 2;
  static const int _legacySchemaVersion = 1;
  static const Set<String> _legacyFieldsWithIntent = <String>{
    'operationKey',
    'intentId',
    'expiresAt',
    'kind',
    'targetId',
    'action',
    'baselineSeq',
  };
  static const Set<String> _legacyFieldsWithoutIntent = <String>{
    'operationKey',
    'expiresAt',
    'kind',
    'targetId',
    'action',
    'baselineSeq',
  };
  static const Set<String> _legacyKinds = <String>{
    'ledger',
    'task',
    'purchase',
  };
  static final RegExp _safeNamespace = RegExp(r'^[A-Za-z0-9_-]{1,200}$');
  static final RegExp _accountId = RegExp(r'^[A-Za-z0-9-]{1,128}$');
  final String _environmentNamespace;
  final FlutterSecureStorage _storage;
  Future<void> _mutationTail = Future<void>.value();

  String _key(String accountId, int schemaVersion) {
    if (!_accountId.hasMatch(accountId)) {
      throw const FormatException('账号标识无效');
    }
    return 'wallet.$_environmentNamespace.pending.v$schemaVersion.$accountId';
  }

  static String _validateNamespace(String value) {
    if (!_safeNamespace.hasMatch(value)) {
      throw const FormatException('环境存储命名空间无效');
    }
    return value;
  }

  @override
  Future<List<WalletPendingMutation>> list(String accountId) async {
    return _serialize(() async {
      return (await _readAll(accountId)).values.toList(growable: false);
    });
  }

  @override
  Future<WalletPendingMutation?> read(
    String accountId,
    String operationKey,
  ) async {
    return _serialize(() async {
      return (await _readAll(accountId))[operationKey];
    });
  }

  @override
  Future<void> write(String accountId, WalletPendingMutation mutation) async {
    await _serialize(() async {
      final Map<String, WalletPendingMutation> records = await _readAll(
        accountId,
      );
      records[mutation.operationKey] = mutation;
      await _writeAll(accountId, records);
    });
  }

  @override
  Future<bool> delete(
    String accountId,
    WalletPendingMutation expectedMutation,
  ) {
    return _serialize(() async {
      final Map<String, WalletPendingMutation> records = await _readAll(
        accountId,
      );
      final WalletPendingMutation? current =
          records[expectedMutation.operationKey];
      if (current == null || !_sameMutation(current, expectedMutation)) {
        return false;
      }
      records.remove(expectedMutation.operationKey);
      await _writeAll(accountId, records);
      return true;
    });
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

  bool _sameMutation(
    WalletPendingMutation current,
    WalletPendingMutation expected,
  ) {
    return current.operationKey == expected.operationKey &&
        current.intentId == expected.intentId &&
        current.expiresAt == expected.expiresAt &&
        current.action == expected.action;
  }

  Future<Map<String, WalletPendingMutation>> _readAll(String accountId) async {
    final String currentKey = _key(accountId, _schemaVersion);
    final String legacyKey = _key(accountId, _legacySchemaVersion);
    final String? currentEncoded = await _storage.read(key: currentKey);
    final String? legacyEncoded = await _storage.read(key: legacyKey);
    if (currentEncoded != null) {
      final Map<String, WalletPendingMutation> current = _decodeCurrent(
        currentEncoded,
      );
      if (legacyEncoded != null) {
        final Map<String, WalletPendingMutation> legacy = _decodeLegacy(
          legacyEncoded,
        );
        _requireLegacyCovered(current, legacy);
        await _storage.delete(key: legacyKey);
      }
      return current;
    }
    if (legacyEncoded == null) {
      return <String, WalletPendingMutation>{};
    }
    final Map<String, WalletPendingMutation> legacy = _decodeLegacy(
      legacyEncoded,
    );
    await _writeAll(accountId, legacy);
    await _storage.delete(key: legacyKey);
    return legacy;
  }

  Map<String, WalletPendingMutation> _decodeCurrent(String encoded) {
    return _decodeRecords(encoded, WalletPendingMutation.fromJson);
  }

  Map<String, WalletPendingMutation> _decodeLegacy(String encoded) {
    return _decodeRecords(encoded, (Map<String, Object?> json) {
      if (WalletPendingMutation._hasExactFields(
        json,
        _legacyFieldsWithoutIntent,
      )) {
        throw const FormatException('旧版待核验积分操作缺少签名请求标识，必须保留并人工核验');
      }
      if (!WalletPendingMutation._hasExactFields(
        json,
        _legacyFieldsWithIntent,
      )) {
        throw const FormatException('旧版待核验积分操作字段无效');
      }
      final Object? kind = json['kind'];
      final Object? targetId = json['targetId'];
      final Object? baselineSeq = json['baselineSeq'];
      if (kind is! String ||
          !_legacyKinds.contains(kind) ||
          targetId is! String ||
          targetId.isEmpty ||
          (baselineSeq != null && baselineSeq is! int)) {
        throw const FormatException('旧版待核验积分操作格式无效');
      }
      return WalletPendingMutation.fromJson(<String, Object?>{
        'operationKey': json['operationKey'],
        'intentId': json['intentId'],
        'expiresAt': json['expiresAt'],
        'action': json['action'],
      });
    });
  }

  Map<String, WalletPendingMutation> _decodeRecords(
    String encoded,
    WalletPendingMutation Function(Map<String, Object?> json) decode,
  ) {
    try {
      final Object? decoded = jsonDecode(encoded);
      if (decoded is! List) {
        throw const FormatException('待核验积分操作存储格式无效');
      }
      final Map<String, WalletPendingMutation> records =
          <String, WalletPendingMutation>{};
      for (final Object? value in decoded) {
        if (value is! Map) {
          throw const FormatException('待核验积分操作存储格式无效');
        }
        final WalletPendingMutation mutation = decode(
          Map<String, Object?>.from(value),
        );
        if (records.containsKey(mutation.operationKey)) {
          throw const FormatException('待核验积分操作包含重复记录');
        }
        records[mutation.operationKey] = mutation;
      }
      return records;
    } on FormatException {
      rethrow;
    } on Object {
      throw const FormatException('待核验积分操作无法解码');
    }
  }

  void _requireLegacyCovered(
    Map<String, WalletPendingMutation> current,
    Map<String, WalletPendingMutation> legacy,
  ) {
    for (final WalletPendingMutation oldRecord in legacy.values) {
      final WalletPendingMutation? newRecord = current[oldRecord.operationKey];
      if (newRecord == null ||
          newRecord.intentId != oldRecord.intentId ||
          newRecord.expiresAt != oldRecord.expiresAt ||
          newRecord.action != oldRecord.action) {
        throw const FormatException('新版存储未完整覆盖旧版待核验积分操作');
      }
    }
  }

  Future<void> _writeAll(
    String accountId,
    Map<String, WalletPendingMutation> records,
  ) async {
    await _storage.write(
      key: _key(accountId, _schemaVersion),
      value: jsonEncode(
        records.values
            .map((WalletPendingMutation mutation) => mutation.toJson())
            .toList(growable: false),
      ),
    );
  }
}
