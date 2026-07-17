import 'dart:async';
import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:dio/dio.dart';
import 'package:uuid/uuid.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/network/session_interceptor.dart';
import '../../auth/data/session_manager.dart';
import 'wallet_pending_mutation_store.dart';
import 'wallet_signer.dart';
import 'wallet_signing_proof.dart';

class WalletMutationCommitted implements Exception {
  const WalletMutationCommitted();
}

class WalletMutationUncertain implements Exception {
  const WalletMutationUncertain();

  @override
  String toString() => '上次积分操作的响应未能确认。客户端已保留同一操作并阻止创建新签名，请刷新后核验签名请求状态。';
}

class WalletSnapshot {
  const WalletSnapshot({
    required this.wallet,
    required this.ledger,
    required this.verification,
    required this.tasks,
    required this.products,
    required this.purchases,
    required this.localKey,
    required this.pendingMutationCount,
  });

  final Wallet wallet;
  final LedgerPage ledger;
  final LedgerVerify verification;
  final TaskPage tasks;
  final ProductPage products;
  final PurchasePage purchases;
  final LocalWalletKey? localKey;
  final int pendingMutationCount;
}

class WalletRepository {
  WalletRepository(
    this._walletApi,
    this._creditApi,
    this._session,
    this._signer, {
    required this._pendingMutationStore,
    Uuid? uuid,
  }) : _uuid = uuid ?? const Uuid();

  final WalletApi _walletApi;
  final CreditApi _creditApi;
  final SessionManager _session;
  final WalletSigner _signer;
  final WalletPendingMutationStore _pendingMutationStore;
  final Uuid _uuid;
  final Set<String> _inFlightOperationKeys = <String>{};
  final Map<String, Future<void>> _walletBindingTails =
      <String, Future<void>>{};
  Future<void> _authorizationTail = Future<void>.value();

  Future<WalletSnapshot> load() async {
    final _WalletSessionSnapshot session = _requireSessionSnapshot();
    final Future<Wallet> wallet = _queryForSession(
      session,
      _walletApi.walletGet,
      '钱包',
    );
    final Future<LedgerPage> ledger = _queryForSession(
      session,
      () => _walletApi.walletLedgerGet(limit: 20),
      '账本',
    );
    final Future<LedgerVerify> verification = _queryForSession(
      session,
      _walletApi.walletLedgerVerifyGet,
      '账本校验',
    );
    final Future<TaskPage> tasks = _queryForSession(
      session,
      () => _creditApi.creditTasksGet(status: 'all', limit: 20),
      '任务',
    );
    final Future<ProductPage> products = _queryForSession(
      session,
      () => _creditApi.creditProductsGet(limit: 20),
      '商品',
    );
    final Future<PurchasePage> purchases = _queryForSession(
      session,
      () => _creditApi.creditPurchasesGet(limit: 20),
      '订单',
    );
    final Future<int> pendingMutationCount = _reconcilePending(session);
    try {
      final LedgerVerify verified = await verification;
      final LocalWalletKey? localKey = await _readLocalKey(session);
      final int unresolvedPending = await pendingMutationCount;
      final Wallet walletData = await wallet;
      final LedgerPage ledgerData = await ledger;
      final TaskPage taskData = await tasks;
      final ProductPage productData = await products;
      final PurchasePage purchaseData = await purchases;
      _assertSessionCurrent(session);
      return WalletSnapshot(
        wallet: walletData,
        ledger: ledgerData,
        verification: verified,
        tasks: taskData,
        products: productData,
        purchases: purchaseData,
        localKey: localKey,
        pendingMutationCount: unresolvedPending,
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on WalletKeyUnavailable catch (failure) {
      throw ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: failure.message,
      );
    }
  }

  Future<LocalWalletKey> createAndBindLocalKey() {
    final _WalletSessionSnapshot session = _requireSessionSnapshot();
    return _serializeWalletBinding(
      session.accountId,
      () => _createAndBindLocalKey(session),
    );
  }

  Future<LocalWalletKey> _createAndBindLocalKey(
    _WalletSessionSnapshot session,
  ) async {
    _assertSessionCurrent(session);
    try {
      final Wallet wallet = await _ownerWallet(session);
      LocalWalletKey? key = await _readLocalKey(session);
      final String? activePublicKey = wallet.activePublicKey;
      if (activePublicKey != null) {
        _validateActivePublicKey(activePublicKey);
        if (key == null) {
          throw const WalletKeyUnavailable('服务端已绑定钱包公钥，但本机没有对应私钥；已禁止生成替代密钥');
        }
        if (key.publicKeyBase64 != activePublicKey) {
          throw const ApiFailure(
            kind: ApiFailureKind.conflict,
            message: '服务端钱包公钥与本机密钥不一致，已停止绑定',
          );
        }
      } else if (key == null) {
        _assertSessionCurrent(session);
        key = await _signer.generate(session.accountId);
        _assertSessionCurrent(session);
      }
      _assertSessionCurrent(session);
      final SessionRequestBinding requestBinding = _requestBinding(session);
      await _walletApi.walletBindPost(
        walletBindPostRequest: WalletBindPostRequest(
          accountId: session.accountId,
          publicKey: key.publicKeyBase64,
        ),
        headers: requestBinding.headers,
        extra: requestBinding.extra,
      );
      _assertSessionCurrent(session);
      return key;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on WalletKeyUnavailable catch (failure) {
      throw ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: failure.message,
      );
    }
  }

  Future<T> _serializeWalletBinding<T>(
    String accountId,
    Future<T> Function() action,
  ) async {
    final Future<void> previous =
        _walletBindingTails[accountId] ?? Future<void>.value();
    final Completer<void> completion = Completer<void>();
    final Future<void> tail = completion.future;
    _walletBindingTails[accountId] = tail;
    await previous;
    try {
      return await action();
    } finally {
      completion.complete();
      if (identical(_walletBindingTails[accountId], tail)) {
        unawaited(_walletBindingTails.remove(accountId));
      }
    }
  }

  Future<void> deleteLocalKey() async {
    try {
      await _signer.delete(_requireAccountId());
    } on WalletKeyUnavailable catch (failure) {
      throw ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: failure.message,
      );
    }
  }

  Future<WalletClaimChallenge> createClaimChallenge() async {
    try {
      return _requireData(
        await _walletApi.walletClaimChallengeGet(),
        '旧钱包认领挑战',
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<Wallet> claimLegacyWallet({
    required String legacyUserHash,
    required String challengeId,
    required String signature,
  }) async {
    try {
      return _requireData(
        await _walletApi.walletClaimPost(
          walletClaimPostRequest: WalletClaimPostRequest(
            legacyUserHash: legacyUserHash.trim(),
            challengeId: challengeId,
            signature: signature.trim(),
          ),
        ),
        '旧钱包认领',
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> tip(TipInput input) async {
    if (input.amount < 1 ||
        input.toAccountId.trim().isEmpty ||
        input.targetId.trim().isEmpty ||
        input.targetType == TipInputTargetTypeEnum.unknownDefaultOpenApi) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '打赏金额和目标无效',
      );
    }
    final _WalletAuthorization authorization = await _authorize(
      _proofExpectation(
        () => WalletSigningProofExpectation.tip(
          accountId: _requireAccountId(),
          input: input,
        ),
      ),
    );
    try {
      _assertSessionCurrent(authorization.session);
      await _creditApi.creditTipPost(
        xWalletIntent: authorization.intentId,
        xWalletSig: authorization.signature,
        idempotencyKey: authorization.idempotencyKey,
        tipInput: input,
        extra: const <String, Object>{'yourtj.disableSessionRetry': true},
      );
      await _completeMutation(authorization);
    } on DioException catch (exception) {
      await _handleSignedFailure(authorization, exception);
    }
  }

  Future<Task> createTask(TaskInput input) async {
    if (input.rewardAmount < 1 || input.title.trim().isEmpty) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '悬赏标题和奖励金额无效',
      );
    }
    final _WalletAuthorization authorization = await _authorize(
      _proofExpectation(
        () => WalletSigningProofExpectation.taskCreate(
          accountId: _requireAccountId(),
          input: input,
        ),
      ),
    );
    try {
      _assertSessionCurrent(authorization.session);
      final Response<Task> response = await _creditApi.creditTasksPost(
        xWalletIntent: authorization.intentId,
        xWalletSig: authorization.signature,
        idempotencyKey: authorization.idempotencyKey,
        taskInput: input,
        extra: const <String, Object>{'yourtj.disableSessionRetry': true},
      );
      return _completeSignedDataMutation(response, '任务', authorization);
    } on DioException catch (exception) {
      await _handleSignedFailure(authorization, exception);
    }
  }

  Future<Product> createProduct(ProductInput input) async {
    if (input.price < 1 || input.stock < 0 || input.title.trim().isEmpty) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '商品标题、价格或库存无效',
      );
    }
    try {
      return _requireData(
        await _creditApi.creditProductsPost(productInput: input),
        '商品',
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> acceptTask(String taskId) async {
    if (taskId.trim().isEmpty) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '任务标识无效',
      );
    }
    try {
      await _creditApi.creditTasksIdAcceptPost(id: taskId);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> updateTask({
    required Task task,
    required TaskActionActionEnum action,
  }) async {
    final String taskId = task.id;
    final TaskStatusEnum currentStatus = task.status;
    if (taskId.trim().isEmpty ||
        action == TaskActionActionEnum.unknownDefaultOpenApi ||
        currentStatus == TaskStatusEnum.unknownDefaultOpenApi) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '任务操作无效',
      );
    }
    final TaskAction input = TaskAction(action: action);
    final bool requiresWalletAuthorization =
        action != TaskActionActionEnum.submit &&
        !(action == TaskActionActionEnum.delete &&
            currentStatus == TaskStatusEnum.cancelled);
    _WalletAuthorization? authorization;
    if (requiresWalletAuthorization) {
      authorization = await _authorize(
        _proofExpectation(
          () => WalletSigningProofExpectation.taskAction(
            accountId: _requireAccountId(),
            task: task,
            action: action,
          ),
        ),
      );
    }
    try {
      if (authorization != null) {
        _assertSessionCurrent(authorization.session);
      }
      await _creditApi.creditTasksIdActionPost(
        id: taskId,
        taskAction: input,
        xWalletIntent: authorization?.intentId,
        xWalletSig: authorization?.signature,
        idempotencyKey: authorization?.idempotencyKey,
        extra: requiresWalletAuthorization
            ? const <String, Object>{'yourtj.disableSessionRetry': true}
            : null,
      );
      if (authorization != null) {
        await _completeMutation(authorization);
      }
    } on DioException catch (exception) {
      if (authorization != null) {
        await _handleSignedFailure(authorization, exception);
      }
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<Purchase> purchaseProduct(Product product) async {
    final String productId = product.id;
    if (productId.trim().isEmpty ||
        product.price < 1 ||
        product.stock < 1 ||
        product.sellerId.trim().isEmpty ||
        product.status != ProductStatusEnum.onSale) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '页面展示的商品信息不完整或不可购买',
      );
    }
    final _WalletAuthorization authorization = await _authorize(
      _proofExpectation(
        () => WalletSigningProofExpectation.productPurchase(
          accountId: _requireAccountId(),
          product: product,
        ),
      ),
    );
    try {
      _assertSessionCurrent(authorization.session);
      final Response<Purchase> response = await _creditApi
          .creditProductsIdPurchasePost(
            id: productId,
            xWalletIntent: authorization.intentId,
            xWalletSig: authorization.signature,
            idempotencyKey: authorization.idempotencyKey,
            extra: const <String, Object>{'yourtj.disableSessionRetry': true},
          );
      return _completeSignedDataMutation(response, '订单', authorization);
    } on DioException catch (exception) {
      await _handleSignedFailure(authorization, exception);
    }
  }

  Future<void> updatePurchase({
    required Purchase purchase,
    required PurchaseActionActionEnum action,
  }) async {
    final String purchaseId = purchase.id;
    if (purchaseId.trim().isEmpty ||
        action == PurchaseActionActionEnum.unknownDefaultOpenApi) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '订单操作无效',
      );
    }
    final bool movesValue =
        action == PurchaseActionActionEnum.confirm ||
        action == PurchaseActionActionEnum.cancel;
    _WalletAuthorization? authorization;
    if (movesValue) {
      authorization = await _authorize(
        _proofExpectation(
          () => WalletSigningProofExpectation.purchaseAction(
            accountId: _requireAccountId(),
            purchase: purchase,
            action: action,
          ),
        ),
      );
    }
    try {
      if (authorization != null) {
        _assertSessionCurrent(authorization.session);
      }
      await _creditApi.creditPurchasesIdActionPost(
        id: purchaseId,
        purchaseAction: PurchaseAction(action: action),
        xWalletIntent: authorization?.intentId,
        xWalletSig: authorization?.signature,
        idempotencyKey: authorization?.idempotencyKey,
        extra: movesValue
            ? const <String, Object>{'yourtj.disableSessionRetry': true}
            : null,
      );
      if (authorization != null) {
        await _completeMutation(authorization);
      }
    } on DioException catch (exception) {
      if (authorization != null) {
        await _handleSignedFailure(authorization, exception);
      }
      throw ApiFailure.fromDio(exception);
    }
  }

  WalletSigningProofExpectation _proofExpectation(
    WalletSigningProofExpectation Function() create,
  ) {
    try {
      return create();
    } on FormatException {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '当前账号、实体状态或数值不允许创建钱包签名',
      );
    }
  }

  Map<String, Object> _signingIntentRequest(Map<String, Object?> request) {
    final Map<String, Object> result = <String, Object>{};
    for (final MapEntry<String, Object?> entry in request.entries) {
      final Object? value = entry.value;
      if (value == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '钱包签名请求包含无法编码的空值，已停止创建签名',
        );
      }
      result[entry.key] = value;
    }
    return result;
  }

  Future<_WalletAuthorization> _authorize(
    WalletSigningProofExpectation expectation,
  ) async {
    final _WalletSessionSnapshot session = _requireSessionSnapshot();
    final String operationKey = await _operationKey(
      session.accountId,
      expectation.action,
      expectation.normalizedRequest,
    );
    _assertSessionCurrent(session);
    if (!_inFlightOperationKeys.add(operationKey)) {
      throw const WalletMutationUncertain();
    }
    try {
      return await _serializeAuthorization(
        () => _authorizeSerially(
          expectation,
          session: session,
          operationKey: operationKey,
        ),
      );
    } on Object {
      _inFlightOperationKeys.remove(operationKey);
      rethrow;
    }
  }

  Future<_WalletAuthorization> _authorizeSerially(
    WalletSigningProofExpectation expectation, {
    required _WalletSessionSnapshot session,
    required String operationKey,
  }) async {
    _assertSessionCurrent(session);
    final WalletPendingMutation? existing = await _readPendingMutation(
      session,
      operationKey,
    );
    if (existing != null) {
      final SigningIntentOutcomeStatusEnum? outcome = await _intentOutcome(
        session,
        existing,
      );
      _assertSessionCurrent(session);
      if (outcome == SigningIntentOutcomeStatusEnum.committed) {
        await _deletePending(session, existing);
        throw const WalletMutationCommitted();
      }
      if (outcome != SigningIntentOutcomeStatusEnum.expired) {
        throw const WalletMutationUncertain();
      }
      if (!await _deletePending(session, existing)) {
        throw const WalletMutationUncertain();
      }
    }
    try {
      final _VerifiedWallet verifiedWallet = await _verifyActiveWallet(session);
      final String expectedPublicKey = verifiedWallet.publicKey;
      final WalletSigningProofExpectation boundExpectation = _proofExpectation(
        () => expectation.withOwnerBalance(verifiedWallet.balance),
      );
      final SigningIntentInputActionEnum action = boundExpectation.action;
      final String idempotencyKey = 'credit:${_uuid.v4()}';
      final SigningIntent intent = await _queryForSession(
        session,
        () => _creditApi.creditSigningIntentsPost(
          idempotencyKey: idempotencyKey,
          signingIntentInput: SigningIntentInput(
            action: action,
            request: _signingIntentRequest(boundExpectation.request),
          ),
        ),
        '签名请求',
      );
      final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
      if (intent.intentId.isEmpty ||
          intent.signingBytes.isEmpty ||
          intent.expiresAt <= now) {
        throw const ApiFailure(
          kind: ApiFailureKind.conflict,
          message: '钱包签名请求不完整或已过期，请重新确认操作',
        );
      }
      if (!await WalletSigningProofVerifier.matches(
        signingBytes: intent.signingBytes,
        accountId: session.accountId,
        publicKey: expectedPublicKey,
        expiresAt: intent.expiresAt,
        idempotencyKey: idempotencyKey,
        intentId: intent.intentId,
        expectation: boundExpectation,
      )) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '钱包签名请求与当前账号、公钥或操作上下文不匹配',
        );
      }
      _assertSessionCurrent(session);
      final String signature = await _signer.signExactBytes(
        session.accountId,
        expectedPublicKey,
        intent.signingBytes,
      );
      _assertSessionCurrent(session);
      final WalletPendingMutation pendingMutation = WalletPendingMutation(
        operationKey: operationKey,
        intentId: intent.intentId,
        expiresAt: intent.expiresAt,
        action: action.value,
      );
      final _WalletAuthorization authorization = _WalletAuthorization(
        operationKey: operationKey,
        session: session,
        idempotencyKey: idempotencyKey,
        intentId: intent.intentId,
        signature: signature,
        pendingMutation: pendingMutation,
      );
      await _savePendingMutation(session, pendingMutation);
      _assertSessionCurrent(session);
      return authorization;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on WalletKeyUnavailable catch (failure) {
      throw ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: failure.message,
      );
    } on FormatException {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法安全保存待核验的积分操作，已停止提交',
      );
    }
  }

  Future<T> _serializeAuthorization<T>(Future<T> Function() action) async {
    final Completer<void> completion = Completer<void>();
    final Future<void> previous = _authorizationTail;
    _authorizationTail = completion.future;
    await previous;
    try {
      return await action();
    } finally {
      completion.complete();
    }
  }

  Future<void> _completeMutation(_WalletAuthorization authorization) async {
    try {
      _assertSessionCurrent(authorization.session);
      if (!await _deletePending(
        authorization.session,
        authorization.pendingMutation,
      )) {
        throw const WalletMutationCommitted();
      }
    } finally {
      _inFlightOperationKeys.remove(authorization.operationKey);
    }
  }

  Future<T> _completeSignedDataMutation<T>(
    Response<T> response,
    String name,
    _WalletAuthorization authorization,
  ) async {
    final T? data = response.data;
    if (data == null) {
      return _handleSignedFailure(
        authorization,
        DioException(
          requestOptions: response.requestOptions,
          response: response,
          type: DioExceptionType.unknown,
          error: ApiFailure(
            kind: ApiFailureKind.unexpected,
            message: '$name响应不完整，已转入签名请求状态核验',
          ),
        ),
      );
    }
    await _completeMutation(authorization);
    return data;
  }

  Future<Never> _handleSignedFailure(
    _WalletAuthorization authorization,
    DioException exception,
  ) async {
    try {
      _assertSessionCurrent(authorization.session);
      final int? statusCode = exception.response?.statusCode;
      final bool isDefinitiveRejection =
          statusCode != null && statusCode >= 400 && statusCode < 500;
      if (isDefinitiveRejection) {
        if (!await _deletePending(
          authorization.session,
          authorization.pendingMutation,
        )) {
          throw const WalletMutationUncertain();
        }
        throw ApiFailure.fromDio(exception);
      }
      final SigningIntentOutcomeStatusEnum? outcome = await _intentOutcome(
        authorization.session,
        authorization.pendingMutation,
      );
      _assertSessionCurrent(authorization.session);
      if (outcome == SigningIntentOutcomeStatusEnum.committed) {
        await _completeMutation(authorization);
        throw const WalletMutationCommitted();
      }
      if (outcome == SigningIntentOutcomeStatusEnum.expired) {
        if (!await _deletePending(
          authorization.session,
          authorization.pendingMutation,
        )) {
          throw const WalletMutationUncertain();
        }
        throw ApiFailure.fromDio(exception);
      }
      throw const WalletMutationUncertain();
    } finally {
      _inFlightOperationKeys.remove(authorization.operationKey);
    }
  }

  Future<SigningIntentOutcomeStatusEnum?> _intentOutcome(
    _WalletSessionSnapshot session,
    WalletPendingMutation pending,
  ) async {
    _assertSessionCurrent(session);
    try {
      final SigningIntentOutcome outcome = await _queryForSession(
        session,
        () => _creditApi.creditSigningIntentOutcomePost(
          signingIntentOutcomeInput: SigningIntentOutcomeInput(
            intentId: pending.intentId,
          ),
        ),
        '签名请求状态',
      );
      if (outcome.intentId != pending.intentId ||
          outcome.expiresAt != pending.expiresAt ||
          outcome.status ==
              SigningIntentOutcomeStatusEnum.unknownDefaultOpenApi) {
        return null;
      }
      return outcome.status;
    } on DioException {
      return null;
    } on ApiFailure catch (failure) {
      if (failure.kind == ApiFailureKind.cancelled) {
        rethrow;
      }
      return null;
    }
  }

  Future<_VerifiedWallet> _verifyActiveWallet(
    _WalletSessionSnapshot session,
  ) async {
    final Wallet wallet = await _ownerWallet(session);
    final String? activePublicKey = wallet.activePublicKey;
    if (activePublicKey == null) {
      throw const WalletKeyUnavailable('服务端尚未绑定当前账号的钱包公钥');
    }
    _validateActivePublicKey(activePublicKey);
    final LocalWalletKey? localKey = await _readLocalKey(session);
    if (localKey == null) {
      throw const WalletKeyUnavailable('本机没有该账号的钱包密钥');
    }
    if (localKey.publicKeyBase64 != activePublicKey) {
      throw const ApiFailure(
        kind: ApiFailureKind.conflict,
        message: '服务端钱包公钥与本机密钥不一致，已停止创建签名',
      );
    }
    return _VerifiedWallet(publicKey: activePublicKey, balance: wallet.balance);
  }

  Future<Wallet> _ownerWallet(_WalletSessionSnapshot session) async {
    final Wallet wallet = await _queryForSession(
      session,
      _walletApi.walletGet,
      '钱包公钥',
    );
    if (wallet.accountId != session.accountId) {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '钱包响应与当前账号不匹配，已停止创建签名',
      );
    }
    return wallet;
  }

  void _validateActivePublicKey(String activePublicKey) {
    if (!_isCanonicalEd25519PublicKey(activePublicKey)) {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '服务端钱包公钥格式无效，已停止创建签名',
      );
    }
  }

  bool _isCanonicalEd25519PublicKey(String value) {
    try {
      final List<int> decoded = base64Decode(value);
      return decoded.length == 32 && base64Encode(decoded) == value;
    } on FormatException {
      return false;
    }
  }

  Future<String> _operationKey(
    String accountId,
    SigningIntentInputActionEnum action,
    Map<String, Object?> request,
  ) async {
    final String canonical = jsonEncode(<String, Object?>{
      'accountId': accountId,
      'action': action.value,
      'request': _sortedJson(request),
    });
    final Hash digest = await Sha256().hash(utf8.encode(canonical));
    return 'sha256:${base64UrlEncode(digest.bytes).replaceAll('=', '')}';
  }

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

  _WalletSessionSnapshot _requireSessionSnapshot() {
    final current = _session.state;
    final String? accountId = current.account?.id;
    final String? accessToken = _session.accessToken;
    if (!current.isAuthenticated ||
        accountId == null ||
        accessToken == null ||
        accessToken.isEmpty) {
      throw const ApiFailure(
        kind: ApiFailureKind.unauthorized,
        message: '请先登录后使用积分钱包',
      );
    }
    return _WalletSessionSnapshot(
      accountId: accountId,
      accessToken: accessToken,
      generation: current.generation,
    );
  }

  SessionRequestBinding _requestBinding(_WalletSessionSnapshot session) {
    _assertSessionCurrent(session);
    return SessionRequestBinding(
      accountId: session.accountId,
      accessToken: session.accessToken,
      generation: session.generation,
    );
  }

  String _requireAccountId() => _requireSessionSnapshot().accountId;

  bool _isSessionCurrent(_WalletSessionSnapshot session) {
    final current = _session.state;
    return current.isAuthenticated &&
        current.account?.id == session.accountId &&
        _session.accessToken == session.accessToken &&
        current.generation == session.generation;
  }

  void _assertSessionCurrent(_WalletSessionSnapshot session) {
    if (!_isSessionCurrent(session)) {
      throw const ApiFailure(
        kind: ApiFailureKind.cancelled,
        message: '账号已切换，已停止旧账号的积分操作',
      );
    }
  }

  Future<LocalWalletKey?> _readLocalKey(_WalletSessionSnapshot session) async {
    _assertSessionCurrent(session);
    final LocalWalletKey? key = await _signer.readPublicKey(session.accountId);
    _assertSessionCurrent(session);
    return key;
  }

  Future<T> _queryForSession<T>(
    _WalletSessionSnapshot session,
    Future<Response<T>> Function() query,
    String name,
  ) async {
    _assertSessionCurrent(session);
    final Response<T> response = await query();
    _assertSessionCurrent(session);
    return _requireData(response, name);
  }

  Future<WalletPendingMutation?> _readPendingMutation(
    _WalletSessionSnapshot session,
    String operationKey,
  ) async {
    _assertSessionCurrent(session);
    final WalletPendingMutation? pending;
    try {
      pending = await _pendingMutationStore.read(
        session.accountId,
        operationKey,
      );
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '待核验积分操作的安全记录损坏，已阻止创建新签名',
      );
    }
    _assertSessionCurrent(session);
    return pending;
  }

  Future<void> _savePendingMutation(
    _WalletSessionSnapshot session,
    WalletPendingMutation mutation,
  ) async {
    _assertSessionCurrent(session);
    try {
      await _pendingMutationStore.write(session.accountId, mutation);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法安全保存待核验的积分操作，已停止提交',
      );
    }
    _assertSessionCurrent(session);
  }

  Future<bool> _deletePending(
    _WalletSessionSnapshot session,
    WalletPendingMutation mutation,
  ) async {
    _assertSessionCurrent(session);
    try {
      final bool deleted = await _pendingMutationStore.delete(
        session.accountId,
        mutation,
      );
      if (!deleted) {
        return false;
      }
    } on Object {
      return false;
    }
    if (_isSessionCurrent(session)) {
      return true;
    }
    try {
      await _pendingMutationStore.write(session.accountId, mutation);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '账号切换时无法恢复待核验的积分记录，已停止钱包写入',
      );
    }
    _assertSessionCurrent(session);
    return false;
  }

  Future<int> _reconcilePending(_WalletSessionSnapshot session) async {
    _assertSessionCurrent(session);
    final List<WalletPendingMutation> pending;
    try {
      pending = await _pendingMutationStore.list(session.accountId);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法读取待核验的积分操作，已停止钱包写入',
      );
    }
    _assertSessionCurrent(session);
    int unresolved = 0;
    for (final WalletPendingMutation mutation in pending) {
      _assertSessionCurrent(session);
      final SigningIntentOutcomeStatusEnum? outcome = await _intentOutcome(
        session,
        mutation,
      );
      _assertSessionCurrent(session);
      final bool canClear =
          outcome == SigningIntentOutcomeStatusEnum.committed ||
          outcome == SigningIntentOutcomeStatusEnum.expired;
      if (!canClear || !await _deletePending(session, mutation)) {
        unresolved += 1;
      }
    }
    return unresolved;
  }

  T _requireData<T>(Response<T> response, String name) {
    final T? data = response.data;
    if (data == null) {
      throw ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '$name响应不完整，请稍后重试',
      );
    }
    return data;
  }
}

class _WalletAuthorization {
  const _WalletAuthorization({
    required this.operationKey,
    required this.idempotencyKey,
    required this.session,
    required this.intentId,
    required this.signature,
    required this.pendingMutation,
  });

  final String operationKey;
  final _WalletSessionSnapshot session;
  final String idempotencyKey;
  final String intentId;
  final String signature;
  final WalletPendingMutation pendingMutation;
}

class _WalletSessionSnapshot {
  const _WalletSessionSnapshot({
    required this.accountId,
    required this.accessToken,
    required this.generation,
  });

  final String accountId;
  final String accessToken;
  final int generation;
}

class _VerifiedWallet {
  const _VerifiedWallet({required this.publicKey, required this.balance});

  final String publicKey;
  final int balance;
}
