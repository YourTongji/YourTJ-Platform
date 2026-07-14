import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_pending_mutation_store.dart';

void main() {
  const FlutterSecureStorage secureStorage = FlutterSecureStorage();

  setUp(() {
    FlutterSecureStorage.setMockInitialValues(<String, String>{});
  });

  test('persists reconciliation records across store instances', () async {
    final KeychainKeystoreWalletPendingMutationStore first =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_a',
          storage: secureStorage,
        );
    const WalletPendingMutation mutation = WalletPendingMutation(
      operationKey: 'tip-operation',
      expiresAt: 2000000000,
      kind: WalletReconciliationKind.ledger,
      targetId: 'ledger-transaction',
      action: 'credit.tip',
      baselineSeq: 42,
    );

    await first.write('account-1', mutation);

    final KeychainKeystoreWalletPendingMutationStore restored =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_a',
          storage: secureStorage,
        );
    final WalletPendingMutation? record = await restored.read(
      'account-1',
      mutation.operationKey,
    );
    expect(record?.targetId, mutation.targetId);
    expect(record?.baselineSeq, mutation.baselineSeq);
    expect(record?.kind, WalletReconciliationKind.ledger);
  });

  test('isolates records by environment and account', () async {
    final KeychainKeystoreWalletPendingMutationStore environmentA =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_a',
          storage: secureStorage,
        );
    final KeychainKeystoreWalletPendingMutationStore environmentB =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_b',
          storage: secureStorage,
        );
    const WalletPendingMutation mutation = WalletPendingMutation(
      operationKey: 'task-operation',
      expiresAt: 2000000000,
      kind: WalletReconciliationKind.task,
      targetId: 'task-1',
      action: 'cancel',
    );

    await environmentA.write('account-1', mutation);

    expect(await environmentA.read('account-2', mutation.operationKey), isNull);
    expect(await environmentB.read('account-1', mutation.operationKey), isNull);
  });

  test('serializes concurrent updates without dropping a record', () async {
    final KeychainKeystoreWalletPendingMutationStore store =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_a',
          storage: secureStorage,
        );
    const WalletPendingMutation first = WalletPendingMutation(
      operationKey: 'first',
      expiresAt: 2000000000,
      kind: WalletReconciliationKind.task,
      targetId: 'task-1',
      action: 'confirm',
    );
    const WalletPendingMutation second = WalletPendingMutation(
      operationKey: 'second',
      expiresAt: 2000000000,
      kind: WalletReconciliationKind.purchase,
      targetId: 'purchase-1',
      action: 'cancel',
    );

    await Future.wait(<Future<void>>[
      store.write('account-1', first),
      store.write('account-1', second),
    ]);

    final List<WalletPendingMutation> records = await store.list('account-1');
    expect(
      records.map((WalletPendingMutation value) => value.operationKey),
      unorderedEquals(<String>['first', 'second']),
    );
  });

  test('fails closed when a stored record is malformed', () async {
    await secureStorage.write(
      key: 'wallet.environment_a.pending.v1.account-1',
      value: '[{"operationKey":"broken"}]',
    );
    final KeychainKeystoreWalletPendingMutationStore store =
        KeychainKeystoreWalletPendingMutationStore(
          environmentNamespace: 'environment_a',
          storage: secureStorage,
        );

    expect(() => store.list('account-1'), throwsA(isA<FormatException>()));
  });
}
