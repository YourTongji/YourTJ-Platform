import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:yourtj_api/yourtj_api.dart';

/// The exact user-visible state and request semantics a wallet intent may sign.
class WalletSigningProofExpectation {
  WalletSigningProofExpectation._({
    required this.action,
    required Map<String, Object?> request,
    required Map<String, Object?> normalizedRequest,
    required Map<String, Object?> snapshot,
    required this._ledgerEntry,
    required this._requiresOwnerBalance,
  }) : request = Map<String, Object?>.unmodifiable(request),
       normalizedRequest = Map<String, Object?>.unmodifiable(normalizedRequest),
       snapshot = Map<String, Object?>.unmodifiable(snapshot);

  factory WalletSigningProofExpectation.tip({
    required String accountId,
    required TipInput input,
  }) {
    final String actorId = _requiredCanonicalId(accountId, '账号标识');
    final String toAccountId = _requiredCanonicalId(
      input.toAccountId,
      '收款账号标识',
    );
    final String targetId = _requiredCanonicalId(input.targetId, '打赏目标标识');
    final int amount = _requiredPositiveInt(input.amount, '打赏金额');
    if (toAccountId == actorId) {
      throw const FormatException('不能签名自我打赏');
    }
    if (input.targetType != TipInputTargetTypeEnum.review &&
        input.targetType != TipInputTargetTypeEnum.thread &&
        input.targetType != TipInputTargetTypeEnum.comment) {
      throw const FormatException('打赏目标类型无效');
    }
    final Map<String, Object?> request = <String, Object?>{
      'toAccountId': toAccountId,
      'amount': amount,
      'targetType': input.targetType.value,
      'targetId': targetId,
    };
    return WalletSigningProofExpectation._(
      action: SigningIntentInputActionEnum.creditPeriodTip,
      request: request,
      normalizedRequest: request,
      snapshot: const <String, Object?>{},
      ledgerEntry: _WalletLedgerExpectation(
        type: 'tip',
        fromAccount: actorId,
        toAccount: toAccountId,
        amount: amount,
        metadata: <String, Object?>{
          'target_type': input.targetType.value,
          'target_id': targetId,
        },
      ),
      requiresOwnerBalance: true,
    );
  }

  factory WalletSigningProofExpectation.taskCreate({
    required String accountId,
    required TaskInput input,
  }) {
    final String actorId = _requiredCanonicalId(accountId, '账号标识');
    final int rewardAmount = _requiredPositiveInt(input.rewardAmount, '任务奖励');
    final Map<String, Object?> request = <String, Object?>{
      'title': input.title,
      if (input.description != null) 'description': input.description,
      'rewardAmount': rewardAmount,
      if (input.contactInfo != null) 'contactInfo': input.contactInfo,
    };
    final Map<String, Object?> normalizedRequest = <String, Object?>{
      'title': input.title,
      'description': input.description,
      'rewardAmount': rewardAmount,
      'contactInfo': input.contactInfo,
    };
    return WalletSigningProofExpectation._(
      action: SigningIntentInputActionEnum.creditPeriodTaskPeriodCreate,
      request: request,
      normalizedRequest: normalizedRequest,
      snapshot: const <String, Object?>{},
      ledgerEntry: _WalletLedgerExpectation(
        type: 'escrow_hold',
        fromAccount: actorId,
        toAccount: null,
        amount: rewardAmount,
        metadata: const <String, Object?>{},
      ),
      requiresOwnerBalance: true,
    );
  }

  factory WalletSigningProofExpectation.productPurchase({
    required String accountId,
    required Product product,
  }) {
    final String actorId = _requiredCanonicalId(accountId, '账号标识');
    final String productId = _requiredCanonicalId(product.id, '商品标识');
    final String sellerId = _requiredCanonicalId(product.sellerId, '卖家标识');
    final int price = _requiredPositiveInt(product.price, '商品价格');
    final int stock = _requiredPositiveInt(product.stock, '商品库存');
    final ProductStatusEnum status = product.status;
    if (status != ProductStatusEnum.onSale || sellerId == actorId) {
      throw const FormatException('商品状态无效');
    }
    final Map<String, Object?> request = <String, Object?>{
      'productId': productId,
    };
    return WalletSigningProofExpectation._(
      action: SigningIntentInputActionEnum.creditPeriodProductPeriodPurchase,
      request: request,
      normalizedRequest: request,
      snapshot: <String, Object?>{
        'price': price,
        'stock': stock,
        'status': status.value,
        'sellerId': sellerId,
      },
      ledgerEntry: _WalletLedgerExpectation(
        type: 'escrow_hold',
        fromAccount: actorId,
        toAccount: null,
        amount: price,
        metadata: <String, Object?>{'product_id': productId},
      ),
      requiresOwnerBalance: false,
    );
  }

  factory WalletSigningProofExpectation.taskAction({
    required String accountId,
    required Task task,
    required TaskActionActionEnum action,
  }) {
    final String actorId = _requiredCanonicalId(accountId, '账号标识');
    final String taskId = _requiredCanonicalId(task.id, '任务标识');
    final String creatorId = _requiredCanonicalId(task.creatorId, '任务发起人');
    final String? acceptorId = task.acceptorId == null
        ? null
        : _requiredCanonicalId(task.acceptorId, '任务接单人');
    final int amount = _requiredPositiveInt(task.rewardAmount, '任务奖励');
    final TaskStatusEnum status = task.status;
    if (!_taskActionAllowed(
      actorId: actorId,
      creatorId: creatorId,
      acceptorId: acceptorId,
      status: status,
      action: action,
    )) {
      throw const FormatException('任务状态或操作无效');
    }
    final Map<String, Object?> request = <String, Object?>{
      'id': taskId,
      'action': action.value,
    };
    return WalletSigningProofExpectation._(
      action: SigningIntentInputActionEnum.creditPeriodTaskPeriodAction,
      request: request,
      normalizedRequest: request,
      snapshot: <String, Object?>{
        'status': status.value,
        'partyA': creatorId,
        'partyB': acceptorId ?? '0',
        'amount': amount,
        'actorId': actorId,
      },
      ledgerEntry: null,
      requiresOwnerBalance: false,
    );
  }

  factory WalletSigningProofExpectation.purchaseAction({
    required String accountId,
    required Purchase purchase,
    required PurchaseActionActionEnum action,
  }) {
    final String actorId = _requiredCanonicalId(accountId, '账号标识');
    final String purchaseId = _requiredCanonicalId(purchase.id, '订单标识');
    final String buyerId = _requiredCanonicalId(purchase.buyerId, '买家标识');
    final String sellerId = _requiredCanonicalId(purchase.sellerId, '卖家标识');
    final int amount = _requiredPositiveInt(purchase.amount, '订单金额');
    final PurchaseStatusEnum status = purchase.status;
    if (!_purchaseActionAllowed(
      actorId: actorId,
      buyerId: buyerId,
      status: status,
      action: action,
    )) {
      throw const FormatException('订单状态或操作无效');
    }
    final Map<String, Object?> request = <String, Object?>{
      'id': purchaseId,
      'action': action.value,
    };
    return WalletSigningProofExpectation._(
      action: SigningIntentInputActionEnum.creditPeriodPurchasePeriodAction,
      request: request,
      normalizedRequest: request,
      snapshot: <String, Object?>{
        'status': status.value,
        'partyA': buyerId,
        'partyB': sellerId,
        'amount': amount,
        'actorId': actorId,
      },
      ledgerEntry: null,
      requiresOwnerBalance: false,
    );
  }

  final SigningIntentInputActionEnum action;
  final Map<String, Object?> request;
  final Map<String, Object?> normalizedRequest;
  final Map<String, Object?> snapshot;
  final _WalletLedgerExpectation? _ledgerEntry;
  final bool _requiresOwnerBalance;

  /// Binds balance-based snapshots to the same owner-wallet response used for key verification.
  WalletSigningProofExpectation withOwnerBalance(int balance) {
    if (!_isSafeInteger(balance)) {
      throw const FormatException('钱包余额超出安全整数范围');
    }
    if (_ledgerEntry != null && balance < _ledgerEntry.amount) {
      throw const FormatException('钱包余额无效或不足');
    }
    if (!_requiresOwnerBalance) {
      return this;
    }
    if (_ledgerEntry == null) {
      throw const FormatException('钱包余额无效或不足');
    }
    return WalletSigningProofExpectation._(
      action: action,
      request: request,
      normalizedRequest: normalizedRequest,
      snapshot: <String, Object?>{'balance': balance},
      ledgerEntry: _ledgerEntry,
      requiresOwnerBalance: false,
    );
  }

  static int _requiredPositiveInt(int? value, String name) {
    if (value == null || value < 1 || !_isSafeInteger(value)) {
      throw FormatException('$name无效');
    }
    return value;
  }

  static bool _isSafeInteger(int value) =>
      value >= -9007199254740991 && value <= 9007199254740991;

  static String _requiredCanonicalId(String? value, String name) {
    if (value == null ||
        value.length > 19 ||
        !_canonicalPositiveInteger.hasMatch(value) ||
        BigInt.parse(value) > _maxI64) {
      throw FormatException('$name无效');
    }
    return value;
  }

  static bool _taskActionAllowed({
    required String actorId,
    required String creatorId,
    required String? acceptorId,
    required TaskStatusEnum status,
    required TaskActionActionEnum action,
  }) {
    if (action == TaskActionActionEnum.confirm) {
      return actorId == creatorId &&
          acceptorId != null &&
          status == TaskStatusEnum.submitted;
    }
    if (action == TaskActionActionEnum.cancel) {
      return actorId == creatorId &&
          status != TaskStatusEnum.completed &&
          status != TaskStatusEnum.cancelled &&
          status != TaskStatusEnum.unknownDefaultOpenApi;
    }
    if (action == TaskActionActionEnum.reject) {
      return actorId == acceptorId &&
          (status == TaskStatusEnum.inProgress ||
              status == TaskStatusEnum.submitted);
    }
    return action == TaskActionActionEnum.delete &&
        actorId == creatorId &&
        status == TaskStatusEnum.open;
  }

  static bool _purchaseActionAllowed({
    required String actorId,
    required String buyerId,
    required PurchaseStatusEnum status,
    required PurchaseActionActionEnum action,
  }) {
    if (actorId != buyerId) {
      return false;
    }
    if (action == PurchaseActionActionEnum.confirm) {
      return status == PurchaseStatusEnum.delivered;
    }
    return action == PurchaseActionActionEnum.cancel &&
        (status == PurchaseStatusEnum.pending ||
            status == PurchaseStatusEnum.accepted);
  }

  static final RegExp _canonicalPositiveInteger = RegExp(r'^[1-9][0-9]*$');
  static final BigInt _maxI64 = BigInt.parse('9223372036854775807');
}

/// Validates the full canonical wallet proof before any local private-key use.
abstract final class WalletSigningProofVerifier {
  static const Set<String> _envelopeFields = <String>{
    'version',
    'intentId',
    'accountId',
    'publicKey',
    'action',
    'requestHash',
    'snapshot',
    'ledgerEntry',
    'idempotencyKey',
    'expiresAt',
  };
  static const Set<String> _ledgerFields = <String>{
    'tx_id',
    'type',
    'from_account',
    'to_account',
    'amount',
    'nonce',
    'metadata',
    'signer',
    'timestamp',
  };
  static final RegExp _uuidV4 = RegExp(
    r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
  );
  static const int _maxSafeInteger = 9007199254740991;

  static Future<bool> matches({
    required String signingBytes,
    required String accountId,
    required String publicKey,
    required String idempotencyKey,
    required String intentId,
    required int expiresAt,
    required WalletSigningProofExpectation expectation,
  }) async {
    try {
      final Object? decoded = jsonDecode(signingBytes);
      if (decoded is! Map<String, dynamic> ||
          !_hasExactKeys(decoded, _envelopeFields) ||
          _canonicalJson(decoded) != signingBytes ||
          decoded['version'] is! int ||
          decoded['version'] != 1 ||
          decoded['intentId'] != intentId ||
          !_uuidV4.hasMatch(intentId) ||
          decoded['accountId'] != accountId ||
          decoded['publicKey'] != publicKey ||
          decoded['action'] != expectation.action.value ||
          decoded['idempotencyKey'] != idempotencyKey ||
          decoded['expiresAt'] is! int ||
          decoded['expiresAt'] != expiresAt ||
          !_isSafeInteger(decoded['expiresAt']! as int)) {
        return false;
      }
      final String expectedRequestHash = await _requestHash(
        expectation.normalizedRequest,
      );
      if (decoded['requestHash'] != expectedRequestHash ||
          !_jsonEquals(decoded['snapshot'], expectation.snapshot)) {
        return false;
      }
      return _ledgerMatches(
        decoded['ledgerEntry'],
        accountId: accountId,
        intentId: intentId,
        expiresAt: expiresAt,
        expected: expectation._ledgerEntry,
      );
    } on FormatException {
      return false;
    } on Object {
      return false;
    }
  }

  static bool _ledgerMatches(
    Object? value, {
    required String accountId,
    required String intentId,
    required int expiresAt,
    required _WalletLedgerExpectation? expected,
  }) {
    if (expected == null) {
      return value == null;
    }
    if (value is! Map<String, dynamic> ||
        !_hasExactKeys(value, _ledgerFields)) {
      return false;
    }
    final Object? metadataValue = value['metadata'];
    if (metadataValue is! Map<String, dynamic>) {
      return false;
    }
    final Map<String, Object?> expectedMetadata = <String, Object?>{
      ...expected.metadata,
      'signing_intent_id': intentId,
    };
    final Object? timestamp = value['timestamp'];
    return value['tx_id'] is String &&
        _uuidV4.hasMatch(value['tx_id']! as String) &&
        value['type'] == expected.type &&
        value['from_account'] == expected.fromAccount &&
        value['to_account'] == expected.toAccount &&
        value['amount'] == expected.amount &&
        value['amount'] is int &&
        (value['amount']! as int) > 0 &&
        _isSafeInteger(value['amount']! as int) &&
        value['nonce'] is String &&
        _uuidV4.hasMatch(value['nonce']! as String) &&
        _jsonEquals(metadataValue, expectedMetadata) &&
        value['signer'] == accountId &&
        timestamp is int &&
        timestamp > 0 &&
        timestamp <= expiresAt &&
        _isSafeInteger(timestamp);
  }

  static Future<String> _requestHash(Map<String, Object?> request) async {
    final Hash digest = await Sha256().hash(
      utf8.encode(_canonicalJson(request)),
    );
    return digest.bytes
        .map((int byte) => byte.toRadixString(16).padLeft(2, '0'))
        .join();
  }

  static bool _jsonEquals(Object? left, Object? right) =>
      _canonicalJson(left) == _canonicalJson(right);

  static bool _hasExactKeys(Map<String, dynamic> value, Set<String> expected) =>
      value.length == expected.length && value.keys.every(expected.contains);

  static bool _isSafeInteger(int value) =>
      value >= -_maxSafeInteger && value <= _maxSafeInteger;

  static String _canonicalJson(Object? value) => jsonEncode(_sortedJson(value));

  static Object? _sortedJson(Object? value) {
    if (value is Map) {
      final List<String> keys =
          value.keys
              .map((Object? key) => key.toString())
              .toList(growable: false)
            ..sort();
      return <String, Object?>{
        for (final String key in keys) key: _sortedJson(value[key]),
      };
    }
    if (value is List) {
      return value.map(_sortedJson).toList(growable: false);
    }
    return value;
  }
}

class _WalletLedgerExpectation {
  const _WalletLedgerExpectation({
    required this.type,
    required this.fromAccount,
    required this.toAccount,
    required this.amount,
    required this.metadata,
  });

  final String type;
  final String fromAccount;
  final String? toAccount;
  final int amount;
  final Map<String, Object?> metadata;
}
