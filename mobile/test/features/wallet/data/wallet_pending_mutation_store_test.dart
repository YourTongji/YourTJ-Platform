import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_pending_mutation_store.dart';

void main() {
  const FlutterSecureStorage secureStorage = FlutterSecureStorage();
  const String accountId = 'account-1';
  const String currentKey = 'wallet.environment_a.pending.v2.$accountId';
  const String legacyKey = 'wallet.environment_a.pending.v1.$accountId';

  setUp(() {
    FlutterSecureStorage.setMockInitialValues(<String, String>{});
  });

  test('persists only the fields needed for intent outcome recovery', () async {
    final KeychainKeystoreWalletPendingMutationStore first = _store(
      secureStorage,
    );
    final WalletPendingMutation mutation = _mutation(
      operationKey: _operationKey(1),
      intentId: _intentId(1),
    );

    await first.write(accountId, mutation);

    final String encoded = (await secureStorage.read(key: currentKey))!;
    final List<Object?> decoded = jsonDecode(encoded) as List<Object?>;
    expect(
      (decoded.single as Map).keys,
      unorderedEquals(<String>[
        'operationKey',
        'intentId',
        'expiresAt',
        'action',
      ]),
    );
    final WalletPendingMutation? restored = await _store(
      secureStorage,
    ).read(accountId, mutation.operationKey);
    expect(restored?.intentId, mutation.intentId);
    expect(restored?.action, mutation.action);
  });

  test('isolates records by environment and account', () async {
    final KeychainKeystoreWalletPendingMutationStore environmentA = _store(
      secureStorage,
    );
    final KeychainKeystoreWalletPendingMutationStore environmentB =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_b',
          storage: secureStorage,
        );
    final WalletPendingMutation mutation = _mutation(
      operationKey: _operationKey(2),
      intentId: _intentId(2),
    );

    await environmentA.write(accountId, mutation);

    expect(await environmentA.read('account-2', mutation.operationKey), isNull);
    expect(await environmentB.read(accountId, mutation.operationKey), isNull);
  });

  test('serializes concurrent updates without dropping a record', () async {
    final KeychainKeystoreWalletPendingMutationStore store = _store(
      secureStorage,
    );
    final WalletPendingMutation first = _mutation(
      operationKey: _operationKey(3),
      intentId: _intentId(3),
    );
    final WalletPendingMutation second = _mutation(
      operationKey: _operationKey(4),
      intentId: _intentId(4),
    );

    await Future.wait(<Future<void>>[
      store.write(accountId, first),
      store.write(accountId, second),
    ]);

    final List<WalletPendingMutation> records = await store.list(accountId);
    expect(
      records.map((WalletPendingMutation value) => value.operationKey),
      unorderedEquals(<String>[first.operationKey, second.operationKey]),
    );
  });

  test('does not delete a replacement for the expected mutation', () async {
    final KeychainKeystoreWalletPendingMutationStore store = _store(
      secureStorage,
    );
    final WalletPendingMutation original = _mutation(
      operationKey: _operationKey(9),
      intentId: _intentId(9),
    );
    final WalletPendingMutation replacement = _mutation(
      operationKey: original.operationKey,
      intentId: _intentId(10),
    );
    await store.write(accountId, original);
    await store.write(accountId, replacement);

    expect(await store.delete(accountId, original), isFalse);
    expect(
      (await store.read(accountId, original.operationKey))?.intentId,
      replacement.intentId,
    );
    expect(await store.delete(accountId, replacement), isTrue);
    expect(await store.read(accountId, original.operationKey), isNull);
  });

  test('rejects unknown fields in the current schema', () async {
    await secureStorage.write(
      key: currentKey,
      value: jsonEncode(<Object>[
        <String, Object>{..._mutationJson(5), 'kind': 'ledger'},
      ]),
    );

    await expectLater(
      _store(secureStorage).list(accountId),
      throwsA(isA<FormatException>()),
    );
  });

  test('migrates a complete v1 record without legacy hint fields', () async {
    await secureStorage.write(
      key: legacyKey,
      value: jsonEncode(<Object>[
        <String, Object?>{
          ..._mutationJson(6),
          'kind': 'ledger',
          'targetId': 'ledger-transaction',
          'baselineSeq': 42,
        },
      ]),
    );

    final List<WalletPendingMutation> records = await _store(
      secureStorage,
    ).list(accountId);

    expect(records.single.intentId, _intentId(6));
    expect(await secureStorage.read(key: legacyKey), isNull);
    final List<Object?> current =
        jsonDecode((await secureStorage.read(key: currentKey))!)
            as List<Object?>;
    expect(
      (current.single as Map).keys,
      unorderedEquals(<String>[
        'operationKey',
        'intentId',
        'expiresAt',
        'action',
      ]),
    );
  });

  test(
    'keeps released v1 records that lack an intent id fail closed',
    () async {
      final String encoded = jsonEncode(<Object>[
        <String, Object?>{
          'operationKey': _operationKey(7),
          'expiresAt': 2000000000,
          'kind': 'ledger',
          'targetId': 'ledger-transaction',
          'action': 'credit.tip',
          'baselineSeq': 42,
        },
      ]);
      await secureStorage.write(key: legacyKey, value: encoded);

      await expectLater(
        _store(secureStorage).list(accountId),
        throwsA(isA<FormatException>()),
      );

      expect(await secureStorage.read(key: legacyKey), encoded);
      expect(await secureStorage.read(key: currentKey), isNull);
    },
  );

  test('never accepts a v2 snapshot that omits a legacy operation', () async {
    await secureStorage.write(key: currentKey, value: '[]');
    final String legacy = jsonEncode(<Object>[
      <String, Object?>{
        ..._mutationJson(8),
        'kind': 'task',
        'targetId': 'task-1',
        'baselineSeq': null,
      },
    ]);
    await secureStorage.write(key: legacyKey, value: legacy);

    await expectLater(
      _store(secureStorage).list(accountId),
      throwsA(isA<FormatException>()),
    );

    expect(await secureStorage.read(key: legacyKey), legacy);
    expect(await secureStorage.read(key: currentKey), '[]');
  });
}

KeychainKeystoreWalletPendingMutationStore _store(
  FlutterSecureStorage secureStorage,
) {
  return KeychainKeystoreWalletPendingMutationStore(
    environmentNamespace: 'environment_a',
    storage: secureStorage,
  );
}

WalletPendingMutation _mutation({
  required String operationKey,
  required String intentId,
}) {
  return WalletPendingMutation(
    operationKey: operationKey,
    intentId: intentId,
    expiresAt: 2000000000,
    action: 'credit.tip',
  );
}

Map<String, Object> _mutationJson(int value) => <String, Object>{
  'operationKey': _operationKey(value),
  'intentId': _intentId(value),
  'expiresAt': 2000000000,
  'action': 'credit.tip',
};

String _operationKey(int value) {
  return 'sha256:${value.toRadixString(16).padLeft(64, '0')}';
}

String _intentId(int value) {
  return '00000000-0000-4000-8000-${value.toString().padLeft(12, '0')}';
}
