import 'dart:async';
import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

enum WalletReconciliationKind { ledger, task, purchase }

class WalletPendingMutation {
  const WalletPendingMutation({
    required this.operationKey,
    required this.expiresAt,
    required this.kind,
    required this.targetId,
    required this.action,
    this.baselineSeq,
  });

  final String operationKey;
  final int expiresAt;
  final WalletReconciliationKind kind;
  final String targetId;
  final String action;
  final int? baselineSeq;

  Map<String, Object?> toJson() => <String, Object?>{
    'operationKey': operationKey,
    'expiresAt': expiresAt,
    'kind': kind.name,
    'targetId': targetId,
    'action': action,
    'baselineSeq': baselineSeq,
  };

  factory WalletPendingMutation.fromJson(Map<String, Object?> json) {
    final Object? operationKey = json['operationKey'];
    final Object? expiresAt = json['expiresAt'];
    final Object? kind = json['kind'];
    final Object? targetId = json['targetId'];
    final Object? action = json['action'];
    final Object? baselineSeq = json['baselineSeq'];
    if (operationKey is! String ||
        operationKey.isEmpty ||
        expiresAt is! int ||
        targetId is! String ||
        targetId.isEmpty ||
        action is! String ||
        kind is! String ||
        (baselineSeq != null && baselineSeq is! int)) {
      throw const FormatException('待核验积分操作格式无效');
    }
    final WalletReconciliationKind reconciliationKind = WalletReconciliationKind
        .values
        .firstWhere(
          (WalletReconciliationKind value) => value.name == kind,
          orElse: () => throw const FormatException('待核验积分操作类型无效'),
        );
    return WalletPendingMutation(
      operationKey: operationKey,
      expiresAt: expiresAt,
      kind: reconciliationKind,
      targetId: targetId,
      action: action,
      baselineSeq: baselineSeq as int?,
    );
  }
}

abstract interface class WalletPendingMutationStore {
  Future<List<WalletPendingMutation>> list(String accountId);

  Future<WalletPendingMutation?> read(String accountId, String operationKey);

  Future<void> write(String accountId, WalletPendingMutation mutation);

  Future<void> delete(String accountId, String operationKey);
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

  static const int _schemaVersion = 1;
  static final RegExp _safeNamespace = RegExp(r'^[A-Za-z0-9_-]{1,200}$');
  static final RegExp _accountId = RegExp(r'^[A-Za-z0-9-]{1,128}$');
  final String _environmentNamespace;
  final FlutterSecureStorage _storage;
  Future<void> _mutationTail = Future<void>.value();

  String _key(String accountId) {
    if (!_accountId.hasMatch(accountId)) {
      throw const FormatException('账号标识无效');
    }
    return 'wallet.$_environmentNamespace.pending.v$_schemaVersion.$accountId';
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
  Future<void> delete(String accountId, String operationKey) async {
    await _serialize(() async {
      final Map<String, WalletPendingMutation> records = await _readAll(
        accountId,
      );
      if (records.remove(operationKey) == null) {
        return;
      }
      await _writeAll(accountId, records);
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

  Future<Map<String, WalletPendingMutation>> _readAll(String accountId) async {
    final String? encoded = await _storage.read(key: _key(accountId));
    if (encoded == null) {
      return <String, WalletPendingMutation>{};
    }
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
        final WalletPendingMutation mutation = WalletPendingMutation.fromJson(
          Map<String, Object?>.from(value),
        );
        records[mutation.operationKey] = mutation;
      }
      return records;
    } on FormatException {
      rethrow;
    } on Object {
      throw const FormatException('待核验积分操作无法解码');
    }
  }

  Future<void> _writeAll(
    String accountId,
    Map<String, WalletPendingMutation> records,
  ) async {
    if (records.isEmpty) {
      await _storage.delete(key: _key(accountId));
      return;
    }
    await _storage.write(
      key: _key(accountId),
      value: jsonEncode(
        records.values
            .map((WalletPendingMutation mutation) => mutation.toJson())
            .toList(growable: false),
      ),
    );
  }
}
