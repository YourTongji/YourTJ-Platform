import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_signing_proof.dart';

const String _accountId = '1';
const String _intentId = '00000000-0000-4000-8000-000000000001';
const String _idempotencyKey = 'credit:test';
const String _publicKey = 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=';
const int _expiresAt = 2000000000;

void main() {
  test(
    'normalizes omitted task optionals before checking requestHash',
    () async {
      final WalletSigningProofExpectation expectation =
          WalletSigningProofExpectation.taskCreate(
            accountId: _accountId,
            input: TaskInput(title: '整理笔记', rewardAmount: 10),
          ).withOwnerBalance(100);
      final Map<String, Object?> ledgerEntry = _escrowLedger(
        amount: 10,
        metadata: const <String, Object?>{},
      );
      final String valid = await _signingBytes(
        expectation,
        ledgerEntry: ledgerEntry,
      );

      expect(await _matches(valid, expectation), isTrue);

      final String omittedNullHash = await _signingBytes(
        expectation,
        ledgerEntry: ledgerEntry,
        requestForHash: expectation.request,
      );
      expect(await _matches(omittedNullHash, expectation), isFalse);
    },
  );

  test(
    'binds product purchase proof to the displayed product snapshot',
    () async {
      final WalletSigningProofExpectation expectation =
          WalletSigningProofExpectation.productPurchase(
            accountId: _accountId,
            product: Product(
              id: '7',
              sellerId: '2',
              title: '不进入最小化 proof 的标题',
              description: null,
              price: 25,
              stock: 3,
              status: ProductStatusEnum.onSale,
              createdAt: 1,
            ),
          );
      final Map<String, Object?> ledgerEntry = _escrowLedger(
        amount: 25,
        metadata: const <String, Object?>{'product_id': '7'},
      );

      expect(
        await _matches(
          await _signingBytes(expectation, ledgerEntry: ledgerEntry),
          expectation,
        ),
        isTrue,
      );

      for (final MapEntry<String, Object?> tampering
          in <MapEntry<String, Object?>>[
            const MapEntry<String, Object?>('price', 26),
            const MapEntry<String, Object?>('stock', 4),
            const MapEntry<String, Object?>('status', 'sold_out'),
            const MapEntry<String, Object?>('sellerId', '3'),
          ]) {
        expect(
          await _matches(
            await _signingBytes(
              expectation,
              ledgerEntry: ledgerEntry,
              snapshot: <String, Object?>{
                ...expectation.snapshot,
                tampering.key: tampering.value,
              },
            ),
            expectation,
          ),
          isFalse,
          reason: '${tampering.key} must match the displayed product',
        );
      }

      final Map<String, Object?> wrongProductLedger = <String, Object?>{
        ...ledgerEntry,
        'metadata': <String, Object?>{
          'product_id': '8',
          'signing_intent_id': _intentId,
        },
      };
      expect(
        await _matches(
          await _signingBytes(expectation, ledgerEntry: wrongProductLedger),
          expectation,
        ),
        isFalse,
      );
    },
  );

  test('binds task action status, parties, amount, and actor', () async {
    final WalletSigningProofExpectation expectation =
        WalletSigningProofExpectation.taskAction(
          accountId: _accountId,
          task: Task(
            id: '8',
            creatorId: '1',
            acceptorId: '2',
            title: 'Task',
            description: null,
            rewardAmount: 30,
            contactInfo: null,
            status: TaskStatusEnum.submitted,
            createdAt: 1,
          ),
          action: TaskActionActionEnum.confirm,
        );

    expect(
      await _matches(await _signingBytes(expectation), expectation),
      isTrue,
    );

    for (final MapEntry<String, Object?> tampering
        in <MapEntry<String, Object?>>[
          const MapEntry<String, Object?>('status', 'completed'),
          const MapEntry<String, Object?>('partyA', '3'),
          const MapEntry<String, Object?>('partyB', '3'),
          const MapEntry<String, Object?>('amount', 31),
          const MapEntry<String, Object?>('actorId', '2'),
        ]) {
      expect(
        await _matches(
          await _signingBytes(
            expectation,
            snapshot: <String, Object?>{
              ...expectation.snapshot,
              tampering.key: tampering.value,
            },
          ),
          expectation,
        ),
        isFalse,
        reason: '${tampering.key} must match the displayed task',
      );
    }
    expect(
      await _matches(
        await _signingBytes(
          expectation,
          ledgerEntry: _escrowLedger(
            amount: 30,
            metadata: const <String, Object?>{},
          ),
        ),
        expectation,
      ),
      isFalse,
    );
  });

  test('binds purchase action status, parties, amount, and actor', () async {
    final WalletSigningProofExpectation expectation =
        WalletSigningProofExpectation.purchaseAction(
          accountId: _accountId,
          purchase: Purchase(
            id: '9',
            productId: '7',
            buyerId: '1',
            sellerId: '2',
            amount: 40,
            status: PurchaseStatusEnum.delivered,
            deliveryInfo: null,
            createdAt: 1,
          ),
          action: PurchaseActionActionEnum.confirm,
        );

    expect(
      await _matches(await _signingBytes(expectation), expectation),
      isTrue,
    );

    expect(
      await _matches(
        await _signingBytes(
          expectation,
          snapshot: <String, Object?>{...expectation.snapshot, 'actorId': '2'},
        ),
        expectation,
      ),
      isFalse,
    );
  });

  test('enforces task action actor and state semantics before signing', () {
    expect(() => _taskExpectation(), returnsNormally);
    expect(
      () => _taskExpectation(
        action: TaskActionActionEnum.cancel,
        status: TaskStatusEnum.inProgress,
      ),
      returnsNormally,
    );
    expect(
      () => _taskExpectation(
        accountId: '2',
        action: TaskActionActionEnum.reject,
        status: TaskStatusEnum.inProgress,
      ),
      returnsNormally,
    );
    expect(
      () => _taskExpectation(
        action: TaskActionActionEnum.delete,
        status: TaskStatusEnum.open,
        acceptorId: null,
      ),
      returnsNormally,
    );
    final List<WalletSigningProofExpectation Function()> invalid =
        <WalletSigningProofExpectation Function()>[
          () => _taskExpectation(accountId: '2'),
          () => _taskExpectation(status: TaskStatusEnum.inProgress),
          () => _taskExpectation(acceptorId: null),
          () => _taskExpectation(
            accountId: '2',
            action: TaskActionActionEnum.cancel,
            status: TaskStatusEnum.inProgress,
          ),
          () => _taskExpectation(
            action: TaskActionActionEnum.cancel,
            status: TaskStatusEnum.completed,
          ),
          () => _taskExpectation(
            action: TaskActionActionEnum.reject,
            status: TaskStatusEnum.inProgress,
          ),
          () => _taskExpectation(
            accountId: '2',
            action: TaskActionActionEnum.reject,
            status: TaskStatusEnum.open,
          ),
          () => _taskExpectation(
            accountId: '2',
            action: TaskActionActionEnum.delete,
            status: TaskStatusEnum.open,
          ),
          () => _taskExpectation(
            action: TaskActionActionEnum.delete,
            status: TaskStatusEnum.inProgress,
          ),
          () => _taskExpectation(
            action: TaskActionActionEnum.delete,
            status: TaskStatusEnum.cancelled,
          ),
          () => _taskExpectation(
            action: TaskActionActionEnum.submit,
            status: TaskStatusEnum.inProgress,
          ),
        ];
    for (final WalletSigningProofExpectation Function() create in invalid) {
      expect(create, throwsFormatException);
    }
  });

  test('enforces purchase action actor and state semantics before signing', () {
    expect(() => _purchaseExpectation(), returnsNormally);
    expect(
      () => _purchaseExpectation(
        action: PurchaseActionActionEnum.cancel,
        status: PurchaseStatusEnum.pending,
      ),
      returnsNormally,
    );
    expect(
      () => _purchaseExpectation(
        action: PurchaseActionActionEnum.cancel,
        status: PurchaseStatusEnum.accepted,
      ),
      returnsNormally,
    );

    final List<WalletSigningProofExpectation Function()> invalid =
        <WalletSigningProofExpectation Function()>[
          () => _purchaseExpectation(accountId: '2'),
          () => _purchaseExpectation(status: PurchaseStatusEnum.accepted),
          () => _purchaseExpectation(
            action: PurchaseActionActionEnum.cancel,
            status: PurchaseStatusEnum.delivered,
          ),
          () => _purchaseExpectation(
            action: PurchaseActionActionEnum.accept,
            status: PurchaseStatusEnum.pending,
          ),
          () => _purchaseExpectation(
            action: PurchaseActionActionEnum.deliver,
            status: PurchaseStatusEnum.accepted,
          ),
        ];
    for (final WalletSigningProofExpectation Function() create in invalid) {
      expect(create, throwsFormatException);
    }
  });

  test('requires canonical positive i64 account and entity identifiers', () {
    expect(
      () => _tipExpectation(
        accountId: '9223372036854775807',
        toAccountId: '1',
        targetId: '9223372036854775807',
      ),
      returnsNormally,
    );

    for (final String invalidId in <String>[
      '0',
      '-1',
      '+1',
      '01',
      ' 1',
      '9223372036854775808',
    ]) {
      expect(
        () => _tipExpectation(accountId: invalidId),
        throwsFormatException,
        reason: 'account id $invalidId must be rejected',
      );
      expect(
        () => _tipExpectation(toAccountId: invalidId),
        throwsFormatException,
        reason: 'recipient id $invalidId must be rejected',
      );
      expect(
        () => _tipExpectation(targetId: invalidId),
        throwsFormatException,
        reason: 'target id $invalidId must be rejected',
      );
    }

    expect(
      () => _tipExpectation(toAccountId: _accountId),
      throwsFormatException,
    );
    expect(
      () => _tipExpectation(
        targetType: TipInputTargetTypeEnum.unknownDefaultOpenApi,
      ),
      throwsFormatException,
    );
    expect(
      () => WalletSigningProofExpectation.productPurchase(
        accountId: _accountId,
        product: Product(
          id: '07',
          sellerId: '2',
          title: 'Product',
          description: null,
          price: 1,
          stock: 1,
          status: ProductStatusEnum.onSale,
          createdAt: 1,
        ),
      ),
      throwsFormatException,
    );
    expect(() => _taskExpectation(acceptorId: '02'), throwsFormatException);
    expect(() => _purchaseExpectation(sellerId: '02'), throwsFormatException);
  });

  test('rejects unsafe integers and insufficient owner balance', () {
    const int unsafeInteger = 9007199254740992;
    expect(() => _tipExpectation(amount: unsafeInteger), throwsFormatException);
    expect(
      () => WalletSigningProofExpectation.taskCreate(
        accountId: _accountId,
        input: TaskInput(title: '整理笔记', rewardAmount: unsafeInteger),
      ),
      throwsFormatException,
    );
    expect(
      () => WalletSigningProofExpectation.productPurchase(
        accountId: _accountId,
        product: Product(
          id: '7',
          sellerId: '2',
          title: 'Product',
          description: null,
          price: unsafeInteger,
          stock: 1,
          status: ProductStatusEnum.onSale,
          createdAt: 1,
        ),
      ),
      throwsFormatException,
    );
    expect(
      () => _purchaseExpectation(amount: unsafeInteger),
      throwsFormatException,
    );

    final WalletSigningProofExpectation tip = _tipExpectation(amount: 10);
    expect(() => tip.withOwnerBalance(10), returnsNormally);
    expect(() => tip.withOwnerBalance(9), throwsFormatException);
    expect(() => tip.withOwnerBalance(unsafeInteger), throwsFormatException);
    final WalletSigningProofExpectation product =
        WalletSigningProofExpectation.productPurchase(
          accountId: _accountId,
          product: Product(
            id: '7',
            sellerId: '2',
            title: 'Product',
            description: null,
            price: 25,
            stock: 1,
            status: ProductStatusEnum.onSale,
            createdAt: 1,
          ),
        );
    expect(() => product.withOwnerBalance(25), returnsNormally);
    expect(() => product.withOwnerBalance(24), throwsFormatException);
    expect(
      () => _purchaseExpectation().withOwnerBalance(unsafeInteger),
      throwsFormatException,
    );
  });
}

WalletSigningProofExpectation _tipExpectation({
  String accountId = _accountId,
  String toAccountId = '2',
  String targetId = '10',
  int amount = 1,
  TipInputTargetTypeEnum targetType = TipInputTargetTypeEnum.thread,
}) => WalletSigningProofExpectation.tip(
  accountId: accountId,
  input: TipInput(
    toAccountId: toAccountId,
    amount: amount,
    targetType: targetType,
    targetId: targetId,
  ),
);

WalletSigningProofExpectation _taskExpectation({
  String accountId = _accountId,
  String id = '8',
  String creatorId = _accountId,
  String? acceptorId = '2',
  int amount = 30,
  TaskStatusEnum status = TaskStatusEnum.submitted,
  TaskActionActionEnum action = TaskActionActionEnum.confirm,
}) => WalletSigningProofExpectation.taskAction(
  accountId: accountId,
  task: Task(
    id: id,
    creatorId: creatorId,
    acceptorId: acceptorId,
    title: 'Task',
    description: null,
    rewardAmount: amount,
    contactInfo: null,
    status: status,
    createdAt: 1,
  ),
  action: action,
);

WalletSigningProofExpectation _purchaseExpectation({
  String accountId = _accountId,
  String id = '9',
  String buyerId = _accountId,
  String sellerId = '2',
  int amount = 40,
  PurchaseStatusEnum status = PurchaseStatusEnum.delivered,
  PurchaseActionActionEnum action = PurchaseActionActionEnum.confirm,
}) => WalletSigningProofExpectation.purchaseAction(
  accountId: accountId,
  purchase: Purchase(
    id: id,
    productId: '7',
    buyerId: buyerId,
    sellerId: sellerId,
    amount: amount,
    status: status,
    deliveryInfo: null,
    createdAt: 1,
  ),
  action: action,
);

Future<bool> _matches(
  String signingBytes,
  WalletSigningProofExpectation expectation,
) => WalletSigningProofVerifier.matches(
  signingBytes: signingBytes,
  accountId: _accountId,
  publicKey: _publicKey,
  idempotencyKey: _idempotencyKey,
  intentId: _intentId,
  expiresAt: _expiresAt,
  expectation: expectation,
);

Future<String> _signingBytes(
  WalletSigningProofExpectation expectation, {
  Map<String, Object?>? snapshot,
  Object? ledgerEntry,
  Map<String, Object?>? requestForHash,
}) async {
  final Map<String, Object?> envelope = <String, Object?>{
    'version': 1,
    'intentId': _intentId,
    'accountId': _accountId,
    'publicKey': _publicKey,
    'action': expectation.action.value,
    'requestHash': await _requestHash(
      requestForHash ?? expectation.normalizedRequest,
    ),
    'snapshot': snapshot ?? expectation.snapshot,
    'ledgerEntry': ledgerEntry,
    'idempotencyKey': _idempotencyKey,
    'expiresAt': _expiresAt,
  };
  return _canonicalJson(envelope);
}

Map<String, Object?> _escrowLedger({
  required int amount,
  required Map<String, Object?> metadata,
}) => <String, Object?>{
  'tx_id': '00000000-0000-4000-8000-000000000002',
  'type': 'escrow_hold',
  'from_account': _accountId,
  'to_account': null,
  'amount': amount,
  'nonce': '00000000-0000-4000-8000-000000000003',
  'metadata': <String, Object?>{...metadata, 'signing_intent_id': _intentId},
  'signer': _accountId,
  'timestamp': 1999999700,
};

Future<String> _requestHash(Map<String, Object?> request) async {
  final Hash digest = await Sha256().hash(utf8.encode(_canonicalJson(request)));
  return digest.bytes
      .map((int byte) => byte.toRadixString(16).padLeft(2, '0'))
      .join();
}

String _canonicalJson(Object? value) => jsonEncode(_sortedJson(value));

Object? _sortedJson(Object? value) {
  if (value is Map) {
    final Map<String, Object?> normalized = <String, Object?>{
      for (final MapEntry<Object?, Object?> entry in value.entries)
        entry.key.toString(): entry.value,
    };
    final List<String> keys = normalized.keys.toList(growable: false)..sort();
    return <String, Object?>{
      for (final String key in keys) key: _sortedJson(normalized[key]),
    };
  }
  if (value is List) {
    return value.map(_sortedJson).toList(growable: false);
  }
  return value;
}
