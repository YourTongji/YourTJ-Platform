import 'dart:async';
import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:dio/dio.dart';
import 'package:uuid/uuid.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../auth/data/session_manager.dart';
import 'wallet_pending_mutation_store.dart';
import 'wallet_signer.dart';

class WalletMutationCommitted implements Exception {
  const WalletMutationCommitted();
}

class WalletMutationUncertain implements Exception {
  const WalletMutationUncertain();

  @override
  String toString() => '上次积分操作的响应未能确认。客户端已保留同一操作并阻止创建新签名，请刷新后核验账本或业务状态。';
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
  Future<void> _authorizationTail = Future<void>.value();
  int? _latestVerifiedSeq;
  String? _latestVerifiedAccountId;

  Future<WalletSnapshot> load() async {
    final _WalletSessionSnapshot session = _requireSessionSnapshot();
    final String accountId = session.accountId;
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
      _latestVerifiedAccountId = accountId;
      _latestVerifiedSeq = verified.latestSeq ?? _latestVerifiedSeq;
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

  Future<LocalWalletKey> createAndBindLocalKey() async {
    final _WalletSessionSnapshot session = _requireSessionSnapshot();
    try {
      LocalWalletKey? key = await _readLocalKey(session);
      if (key == null) {
        _assertSessionCurrent(session);
        key = await _signer.generate(session.accountId);
        _assertSessionCurrent(session);
      }
      _assertSessionCurrent(session);
      await _walletApi.walletBindPost(
        walletBindPostRequest: WalletBindPostRequest(
          publicKey: key.publicKeyBase64,
        ),
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
      SigningIntentInputActionEnum.creditPeriodTip,
      input.toJson(),
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
      SigningIntentInputActionEnum.creditPeriodTaskPeriodCreate,
      input.toJson(),
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
      final Task task = _requireData(response, '任务');
      await _completeMutation(authorization);
      return task;
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
    required String taskId,
    required TaskActionActionEnum action,
    required TaskStatusEnum currentStatus,
  }) async {
    if (taskId.trim().isEmpty ||
        action == TaskActionActionEnum.unknownDefaultOpenApi ||
        currentStatus == TaskStatusEnum.unknownDefaultOpenApi) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '任务操作无效',
      );
    }
    final TaskAction input = TaskAction(action: action);
    final bool movesValue =
        action != TaskActionActionEnum.submit &&
        !(action == TaskActionActionEnum.delete &&
            currentStatus == TaskStatusEnum.cancelled);
    _WalletAuthorization? authorization;
    if (movesValue) {
      authorization = await _authorize(
        SigningIntentInputActionEnum.creditPeriodTaskPeriodAction,
        <String, Object>{'id': taskId, 'action': action.toString()},
        reconciliationKind: WalletReconciliationKind.task,
        reconciliationTargetId: taskId,
        reconciliationAction: action.value,
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

  Future<Purchase> purchaseProduct(String productId) async {
    if (productId.trim().isEmpty) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '商品标识无效',
      );
    }
    final _WalletAuthorization authorization = await _authorize(
      SigningIntentInputActionEnum.creditPeriodProductPeriodPurchase,
      <String, Object>{'productId': productId},
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
      final Purchase purchase = _requireData(response, '订单');
      await _completeMutation(authorization);
      return purchase;
    } on DioException catch (exception) {
      await _handleSignedFailure(authorization, exception);
    }
  }

  Future<void> updatePurchase({
    required String purchaseId,
    required PurchaseActionActionEnum action,
  }) async {
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
        SigningIntentInputActionEnum.creditPeriodPurchasePeriodAction,
        <String, Object>{'id': purchaseId, 'action': action.toString()},
        reconciliationKind: WalletReconciliationKind.purchase,
        reconciliationTargetId: purchaseId,
        reconciliationAction: action.value,
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

  Future<_WalletAuthorization> _authorize(
    SigningIntentInputActionEnum action,
    Map<String, Object?> request, {
    WalletReconciliationKind? reconciliationKind,
    String? reconciliationTargetId,
    String? reconciliationAction,
  }) async {
    final _WalletSessionSnapshot session = _requireSessionSnapshot();
    final String operationKey = await _operationKey(
      session.accountId,
      action,
      request,
    );
    _assertSessionCurrent(session);
    if (!_inFlightOperationKeys.add(operationKey)) {
      throw const WalletMutationUncertain();
    }
    try {
      return await _serializeAuthorization(
        () => _authorizeSerially(
          action,
          request,
          session: session,
          operationKey: operationKey,
          reconciliationKind: reconciliationKind,
          reconciliationTargetId: reconciliationTargetId,
          reconciliationAction: reconciliationAction,
        ),
      );
    } on Object {
      _inFlightOperationKeys.remove(operationKey);
      rethrow;
    }
  }

  Future<_WalletAuthorization> _authorizeSerially(
    SigningIntentInputActionEnum action,
    Map<String, Object?> request, {
    required _WalletSessionSnapshot session,
    required String operationKey,
    WalletReconciliationKind? reconciliationKind,
    String? reconciliationTargetId,
    String? reconciliationAction,
  }) async {
    _assertSessionCurrent(session);
    final WalletPendingMutation? existing = await _readPendingMutation(
      session,
      operationKey,
    );
    if (existing != null) {
      final bool? committed = await _tryReconcile(session, existing);
      _assertSessionCurrent(session);
      if (committed == true) {
        await _deletePending(session, existing);
        throw const WalletMutationCommitted();
      }
      final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
      if (committed != false || existing.expiresAt > now) {
        throw const WalletMutationUncertain();
      }
      if (!await _deletePending(session, existing)) {
        throw const WalletMutationUncertain();
      }
    }
    final String idempotencyKey = 'credit:${_uuid.v4()}';
    try {
      final bool expectsLedgerEntry =
          reconciliationKind == null &&
          reconciliationTargetId == null &&
          reconciliationAction == null;
      final int? baselineSeq = expectsLedgerEntry
          ? await _ledgerBaseline(session)
          : null;
      final SigningIntent intent = await _queryForSession(
        session,
        () => _creditApi.creditSigningIntentsPost(
          idempotencyKey: idempotencyKey,
          signingIntentInput: SigningIntentInput(
            action: action,
            request: request,
          ),
        ),
        '签名请求',
      );
      final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
      if (intent.expiresAt <= now) {
        throw const ApiFailure(
          kind: ApiFailureKind.conflict,
          message: '钱包签名请求已过期，请重新确认操作',
        );
      }
      _assertSessionCurrent(session);
      final String signature = await _signer.signExactBytes(
        session.accountId,
        intent.signingBytes,
      );
      _assertSessionCurrent(session);
      final String? expectedLedgerTxId = _ledgerTxId(intent.signingBytes);
      final WalletPendingMutation? pendingMutation = expectsLedgerEntry
          ? expectedLedgerTxId == null || baselineSeq == null
                ? null
                : WalletPendingMutation(
                    operationKey: operationKey,
                    expiresAt: intent.expiresAt,
                    kind: WalletReconciliationKind.ledger,
                    targetId: expectedLedgerTxId,
                    action: action.value,
                    baselineSeq: baselineSeq,
                  )
          : reconciliationKind == null ||
                reconciliationTargetId == null ||
                reconciliationAction == null
          ? null
          : WalletPendingMutation(
              operationKey: operationKey,
              expiresAt: intent.expiresAt,
              kind: reconciliationKind,
              targetId: reconciliationTargetId,
              action: reconciliationAction,
            );
      if (pendingMutation == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '该积分操作缺少可核验的服务端结果标识，已停止提交',
        );
      }
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

  Future<Never> _handleSignedFailure(
    _WalletAuthorization authorization,
    DioException exception,
  ) async {
    try {
      _assertSessionCurrent(authorization.session);
      final int? statusCode = exception.response?.statusCode;
      final bool mayHaveCommitted =
          exception.response == null ||
          (statusCode != null && statusCode >= 500);
      if (!mayHaveCommitted) {
        if (!await _deletePending(
          authorization.session,
          authorization.pendingMutation,
        )) {
          throw const WalletMutationUncertain();
        }
        throw ApiFailure.fromDio(exception);
      }
      final bool? committed = await _tryReconcile(
        authorization.session,
        authorization.pendingMutation,
      );
      _assertSessionCurrent(authorization.session);
      if (committed == true) {
        await _completeMutation(authorization);
        throw const WalletMutationCommitted();
      }
      throw const WalletMutationUncertain();
    } finally {
      _inFlightOperationKeys.remove(authorization.operationKey);
    }
  }

  Future<bool?> _tryReconcile(
    _WalletSessionSnapshot session,
    WalletPendingMutation pending,
  ) async {
    _assertSessionCurrent(session);
    try {
      return switch (pending.kind) {
        WalletReconciliationKind.ledger => _ledgerContains(
          session,
          pending.targetId,
          pending.baselineSeq ?? 0,
        ),
        WalletReconciliationKind.task => _taskReachedState(
          session,
          pending.targetId,
          TaskActionActionEnum.values.firstWhere(
            (TaskActionActionEnum value) => value.value == pending.action,
          ),
        ),
        WalletReconciliationKind.purchase => _purchaseReachedState(
          session,
          pending.targetId,
          PurchaseActionActionEnum.values.firstWhere(
            (PurchaseActionActionEnum value) => value.value == pending.action,
          ),
        ),
      };
    } on DioException {
      return null;
    } on ApiFailure catch (failure) {
      if (failure.kind == ApiFailureKind.cancelled) {
        rethrow;
      }
      return null;
    } on StateError {
      return null;
    }
  }

  Future<int> _ledgerBaseline(_WalletSessionSnapshot session) async {
    _assertSessionCurrent(session);
    final int? known = _latestVerifiedAccountId == session.accountId
        ? _latestVerifiedSeq
        : null;
    if (known != null) {
      return known;
    }
    try {
      final LedgerVerify verification = await _queryForSession(
        session,
        _walletApi.walletLedgerVerifyGet,
        '账本校验',
      );
      final int baseline = verification.latestSeq ?? 0;
      _latestVerifiedAccountId = session.accountId;
      _latestVerifiedSeq = baseline;
      return baseline;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<bool?> _ledgerContains(
    _WalletSessionSnapshot session,
    String txId,
    int baselineSeq,
  ) async {
    String? cursor = baselineSeq > 0 ? baselineSeq.toString() : null;
    for (int pageIndex = 0; pageIndex < 20; pageIndex += 1) {
      final LedgerPage page = await _queryForSession(
        session,
        () => _walletApi.walletLedgerGet(cursor: cursor, limit: 100),
        '账本核验',
      );
      if (page.items.any((LedgerEntry entry) => entry.txId == txId)) {
        return true;
      }
      if (!page.hasMore || page.nextCursor == null) {
        return false;
      }
      cursor = page.nextCursor;
    }
    return null;
  }

  Future<bool?> _taskReachedState(
    _WalletSessionSnapshot session,
    String taskId,
    TaskActionActionEnum action,
  ) async {
    String? cursor;
    for (int pageIndex = 0; pageIndex < 20; pageIndex += 1) {
      final TaskPage page = await _queryForSession(
        session,
        () => _creditApi.creditTasksGet(
          status: 'all',
          cursor: cursor,
          limit: 100,
        ),
        '任务核验',
      );
      final Task? task = _findTask(page.items, taskId);
      if (task != null) {
        final TaskStatusEnum? expected = switch (action) {
          TaskActionActionEnum.confirm => TaskStatusEnum.completed,
          TaskActionActionEnum.cancel ||
          TaskActionActionEnum.reject => TaskStatusEnum.cancelled,
          TaskActionActionEnum.delete => null,
          _ => null,
        };
        return expected != null && task.status == expected;
      }
      if (!page.hasMore || page.nextCursor == null) {
        return action == TaskActionActionEnum.delete;
      }
      cursor = page.nextCursor;
    }
    return null;
  }

  Future<bool?> _purchaseReachedState(
    _WalletSessionSnapshot session,
    String purchaseId,
    PurchaseActionActionEnum action,
  ) async {
    String? cursor;
    for (int pageIndex = 0; pageIndex < 20; pageIndex += 1) {
      final PurchasePage page = await _queryForSession(
        session,
        () => _creditApi.creditPurchasesGet(cursor: cursor, limit: 100),
        '订单核验',
      );
      final Purchase? purchase = _findPurchase(page.items, purchaseId);
      if (purchase != null) {
        final PurchaseStatusEnum? expected = switch (action) {
          PurchaseActionActionEnum.confirm => PurchaseStatusEnum.completed,
          PurchaseActionActionEnum.cancel => PurchaseStatusEnum.cancelled,
          _ => null,
        };
        return expected != null && purchase.status == expected;
      }
      if (!page.hasMore || page.nextCursor == null) {
        return false;
      }
      cursor = page.nextCursor;
    }
    return null;
  }

  Task? _findTask(List<Task> tasks, String id) {
    for (final Task task in tasks) {
      if (task.id == id) {
        return task;
      }
    }
    return null;
  }

  Purchase? _findPurchase(List<Purchase> purchases, String id) {
    for (final Purchase purchase in purchases) {
      if (purchase.id == id) {
        return purchase;
      }
    }
    return null;
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

  String? _ledgerTxId(String signingBytes) {
    try {
      final Object? decoded = jsonDecode(signingBytes);
      if (decoded is! Map) {
        return null;
      }
      final Object? ledgerEntry = decoded['ledgerEntry'];
      if (ledgerEntry is! Map) {
        return null;
      }
      final Object? txId = ledgerEntry['tx_id'];
      return txId is String && txId.isNotEmpty ? txId : null;
    } on FormatException {
      return null;
    }
  }

  _WalletSessionSnapshot _requireSessionSnapshot() {
    final current = _session.state;
    final String? accountId = current.account?.id;
    if (!current.isAuthenticated || accountId == null) {
      throw const ApiFailure(
        kind: ApiFailureKind.unauthorized,
        message: '请先登录后使用积分钱包',
      );
    }
    return _WalletSessionSnapshot(
      accountId: accountId,
      generation: current.generation,
    );
  }

  String _requireAccountId() => _requireSessionSnapshot().accountId;

  bool _isSessionCurrent(_WalletSessionSnapshot session) {
    final current = _session.state;
    return current.isAuthenticated &&
        current.account?.id == session.accountId &&
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
      await _pendingMutationStore.delete(
        session.accountId,
        mutation.operationKey,
      );
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
    final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    for (final WalletPendingMutation mutation in pending) {
      _assertSessionCurrent(session);
      final bool? committed = await _tryReconcile(session, mutation);
      _assertSessionCurrent(session);
      final bool canClear =
          committed == true ||
          (committed == false && mutation.expiresAt <= now);
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
    required this.generation,
  });

  final String accountId;
  final int generation;
}
