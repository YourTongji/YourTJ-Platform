import 'dart:async';
import 'dart:convert';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_pending_mutation_store.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_repository.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_seed_store.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_signer.dart';

import '../../auth/support/session_test_support.dart';

void main() {
  group('WalletRepository uncertain-result recovery', () {
    late _WalletHarness harness;

    setUp(() async {
      harness = _WalletHarness();
      await harness.initialize();
    });

    tearDown(() => harness.dispose());

    test('recognizes a commit after the mutation response is lost', () async {
      harness.tipOutcome = _TipOutcome.lostResponseCommitted;

      await expectLater(
        harness.repository.tip(_tip()),
        throwsA(isA<WalletMutationCommitted>()),
      );

      expect(harness.signingIntentRequests, 1);
      expect(harness.tipRequests, 1);
      expect(await harness.pendingStore.list('account-1'), isEmpty);
      expect(harness.tipIdempotencyKey, harness.signingIdempotencyKey);
    });

    test(
      'persists an unresolved operation and blocks replay after restart',
      () async {
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        final WalletPendingMutation pending = (await harness.pendingStore.list(
          'account-1',
        )).single;
        expect(pending.operationKey, startsWith('sha256:'));
        expect(pending.operationKey, isNot(contains('account-1')));
        expect(pending.operationKey, isNot(contains('ledger-tx-1')));

        harness.restoreRepository();
        harness.ledgerContainsTransaction = true;
        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationCommitted>()),
        );

        expect(harness.signingIntentRequests, 1);
        expect(harness.tipRequests, 1);
        expect(await harness.pendingStore.list('account-1'), isEmpty);
      },
    );

    test(
      'does not create another intent while the result is unresolved',
      () async {
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );

        expect(harness.signingIntentRequests, 1);
        expect(harness.tipRequests, 1);
        expect(await harness.pendingStore.list('account-1'), hasLength(1));
      },
    );

    test(
      'serializes concurrent identical operations before creating an intent',
      () async {
        final List<Object> outcomes =
            await Future.wait<Object>(<Future<Object>>[
              _captureOutcome(harness.repository.tip(_tip())),
              _captureOutcome(harness.repository.tip(_tip())),
            ]);

        expect(harness.signingIntentRequests, 1);
        expect(harness.tipRequests, 1);
        expect(
          outcomes.where((Object outcome) => outcome == 'success'),
          hasLength(1),
        );
        expect(
          outcomes.where(
            (Object outcome) =>
                outcome is WalletMutationCommitted ||
                outcome is WalletMutationUncertain,
          ),
          hasLength(1),
        );
        expect(await harness.pendingStore.list('account-1'), isEmpty);
      },
    );

    test(
      'creates a fresh intent only after an uncommitted record expires',
      () async {
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        final WalletPendingMutation pending = (await harness.pendingStore.list(
          'account-1',
        )).single;
        await harness.pendingStore.write(
          'account-1',
          WalletPendingMutation(
            operationKey: pending.operationKey,
            expiresAt: 0,
            kind: pending.kind,
            targetId: pending.targetId,
            action: pending.action,
            baselineSeq: pending.baselineSeq,
          ),
        );
        harness.tipOutcome = _TipOutcome.success;

        await harness.repository.tip(_tip());

        expect(harness.signingIntentRequests, 2);
        expect(harness.tipRequests, 2);
        expect(await harness.pendingStore.list('account-1'), isEmpty);
      },
    );

    test(
      'clears a rejected operation so a corrected retry can proceed',
      () async {
        harness.tipOutcome = _TipOutcome.rejected;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(
            isA<ApiFailure>().having(
              (ApiFailure failure) => failure.kind,
              'kind',
              ApiFailureKind.invalidInput,
            ),
          ),
        );

        expect(await harness.pendingStore.list('account-1'), isEmpty);
      },
    );

    test('rejects edge amounts before creating a signing intent', () async {
      await expectLater(
        harness.repository.tip(_tip(amount: 0)),
        throwsA(
          isA<ApiFailure>().having(
            (ApiFailure failure) => failure.kind,
            'kind',
            ApiFailureKind.invalidInput,
          ),
        ),
      );

      expect(harness.signingIntentRequests, 0);
      expect(harness.tipRequests, 0);
    });
  });

  group('WalletRepository session isolation', () {
    late _WalletHarness harness;

    setUp(() async {
      harness = _WalletHarness();
      await harness.initialize();
    });

    tearDown(() => harness.dispose());

    test('does not bind account A key after switching to account B', () async {
      final _AsyncGate signerRead = harness.seedStore.pauseNextRead();

      final Future<LocalWalletKey> bind = harness.repository
          .createAndBindLocalKey();
      await signerRead.started.future;
      await harness.switchToAccount2();
      signerRead.release.complete();

      await expectLater(bind, throwsA(_isCancelledFailure));
      expect(harness.bindRequests, 0);
      expect(harness.session.manager.state.account?.id, 'account-2');
    });

    test(
      'does not query account A pending state after switching during store read',
      () async {
        await harness.createUnresolvedPending();
        harness.restoreRepository();
        final int ledgerRequestsBeforeRetry = harness.ledgerRequests;
        final _AsyncGate pendingRead = harness.pendingStore.pauseNextRead();

        final Future<void> retry = harness.repository.tip(_tip());
        await pendingRead.started.future;
        await harness.switchToAccount2();
        pendingRead.release.complete();

        await expectLater(retry, throwsA(_isCancelledFailure));
        expect(harness.ledgerRequests, ledgerRequestsBeforeRetry);
        expect(await harness.pendingStore.list('account-1'), hasLength(1));
      },
    );

    test(
      'discards account A reconciliation result when the session switches in flight',
      () async {
        await harness.createUnresolvedPending(expired: true);
        harness.restoreRepository();
        final _AsyncGate ledgerQuery = harness.pauseNextLedgerQuery();

        final Future<void> retry = harness.repository.tip(_tip());
        await ledgerQuery.started.future;
        await harness.switchToAccount2();
        ledgerQuery.release.complete();

        await expectLater(retry, throwsA(_isCancelledFailure));
        expect(await harness.pendingStore.list('account-1'), hasLength(1));
        expect(harness.signingIntentRequests, 1);
      },
    );

    test(
      'restores account A anti-replay record when the session switches during delete',
      () async {
        await harness.createUnresolvedPending(expired: true);
        harness.restoreRepository();
        final _AsyncGate pendingDelete = harness.pendingStore.pauseNextDelete();

        final Future<void> retry = harness.repository.tip(_tip());
        await pendingDelete.started.future;
        await harness.switchToAccount2();
        pendingDelete.release.complete();

        await expectLater(retry, throwsA(_isCancelledFailure));
        expect(await harness.pendingStore.list('account-1'), hasLength(1));
        expect(harness.signingIntentRequests, 1);
      },
    );
  });
}

final Matcher _isCancelledFailure = isA<ApiFailure>().having(
  (ApiFailure failure) => failure.kind,
  'kind',
  ApiFailureKind.cancelled,
);

Future<Object> _captureOutcome(Future<void> operation) async {
  try {
    await operation;
    return 'success';
  } on Object catch (error) {
    return error;
  }
}

TipInput _tip({int amount = 1}) => TipInput(
  toAccountId: '2',
  amount: amount,
  targetType: TipInputTargetTypeEnum.thread,
  targetId: '10',
);

enum _TipOutcome {
  success,
  rejected,
  lostResponseCommitted,
  lostResponseUnresolved,
}

class _WalletHarness {
  late SessionHarness session;
  final _MemoryWalletSeedStore seedStore = _MemoryWalletSeedStore();
  final _MemoryPendingStore pendingStore = _MemoryPendingStore();
  late WalletSigner signer;
  late WalletRepository repository;
  late YourtjApi api;
  _TipOutcome tipOutcome = _TipOutcome.success;
  bool ledgerContainsTransaction = false;
  int signingIntentRequests = 0;
  int tipRequests = 0;
  int bindRequests = 0;
  int ledgerRequests = 0;
  String? signingIdempotencyKey;
  String? tipIdempotencyKey;
  _AsyncGate? _nextLedgerQuery;

  Future<void> initialize() async {
    session = SessionHarness(
      storage: MemorySessionStorage(
        activeAccountId: 'account-1',
        refreshTokens: <String, String>{'account-1': 'refresh-old'},
      ),
      handler: _handle,
    );
    await session.manager.initialize();
    api = YourtjApi(dio: session.dio, interceptors: const []);
    signer = WalletSigner(seedStore);
    await signer.generate('account-1');
    restoreRepository();
  }

  void restoreRepository() {
    repository = WalletRepository(
      api.getWalletApi(),
      api.getCreditApi(),
      session.manager,
      signer,
      pendingMutationStore: pendingStore,
    );
  }

  Future<void> switchToAccount2() => session.manager.passwordLogin(
    email: 'account-2@tongji.edu.cn',
    password: 'correct horse battery staple',
  );

  Future<WalletPendingMutation> createUnresolvedPending({
    bool expired = false,
  }) async {
    tipOutcome = _TipOutcome.lostResponseUnresolved;
    await expectLater(
      repository.tip(_tip()),
      throwsA(isA<WalletMutationUncertain>()),
    );
    final WalletPendingMutation pending = (await pendingStore.list(
      'account-1',
    )).single;
    if (!expired) {
      return pending;
    }
    final WalletPendingMutation expiredPending = WalletPendingMutation(
      operationKey: pending.operationKey,
      expiresAt: 0,
      kind: pending.kind,
      targetId: pending.targetId,
      action: pending.action,
      baselineSeq: pending.baselineSeq,
    );
    await pendingStore.write('account-1', expiredPending);
    return expiredPending;
  }

  _AsyncGate pauseNextLedgerQuery() {
    final _AsyncGate gate = _AsyncGate();
    _nextLedgerQuery = gate;
    return gate;
  }

  Future<void> dispose() => session.dispose();

  Future<ResponseBody> _handle(RequestOptions options) async {
    final String path = options.uri.path;
    if (path.endsWith('/auth/refresh')) {
      return jsonResponse(
        authTokensJson(
          accountId: 'account-1',
          accessToken: 'access-new',
          refreshToken: 'refresh-new',
        ),
      );
    }
    if (path.endsWith('/auth/password/login')) {
      return jsonResponse(
        authTokensJson(
          accountId: 'account-2',
          accessToken: 'access-account-2',
          refreshToken: 'refresh-account-2',
        ),
      );
    }
    if (path.endsWith('/wallet/bind')) {
      bindRequests += 1;
      return ResponseBody.fromString('', 204);
    }
    if (path.endsWith('/wallet/ledger/verify')) {
      return jsonResponse(<String, Object>{
        'ok': true,
        'latestSeq': 41,
        'latestHash': 'hash-41',
      });
    }
    if (path.endsWith('/credit/signing-intents')) {
      signingIntentRequests += 1;
      signingIdempotencyKey = options.headers['Idempotency-Key'] as String?;
      return jsonResponse(<String, Object>{
        'intentId': '00000000-0000-4000-8000-000000000001',
        'signingBytes': jsonEncode(<String, Object>{
          'ledgerEntry': <String, Object>{'tx_id': 'ledger-tx-1'},
        }),
        'expiresAt': 2000000000,
      });
    }
    if (path.endsWith('/credit/tip')) {
      tipRequests += 1;
      tipIdempotencyKey = options.headers['Idempotency-Key'] as String?;
      switch (tipOutcome) {
        case _TipOutcome.success:
          ledgerContainsTransaction = true;
          return ResponseBody.fromString('', 204);
        case _TipOutcome.rejected:
          return jsonResponse(<String, Object>{
            'error': <String, String>{
              'code': 'INVALID_TARGET',
              'message': '目标无效',
            },
          }, statusCode: 400);
        case _TipOutcome.lostResponseCommitted:
          ledgerContainsTransaction = true;
          throw DioException(
            requestOptions: options,
            type: DioExceptionType.connectionError,
          );
        case _TipOutcome.lostResponseUnresolved:
          throw DioException(
            requestOptions: options,
            type: DioExceptionType.connectionError,
          );
      }
    }
    if (path.endsWith('/wallet/ledger')) {
      ledgerRequests += 1;
      final _AsyncGate? gate = _nextLedgerQuery;
      _nextLedgerQuery = null;
      if (gate != null) {
        gate.started.complete();
        await gate.release.future;
      }
      return jsonResponse(<String, Object?>{
        'items': ledgerContainsTransaction
            ? <Object>[
                <String, Object>{
                  'seq': 42,
                  'txId': 'ledger-tx-1',
                  'type': 'tip',
                  'fromAccount': 'account-1',
                  'toAccount': '2',
                  'amount': 1,
                  'hash': 'hash-42',
                  'createdAt': 1,
                },
              ]
            : <Object>[],
        'nextCursor': null,
        'hasMore': false,
      });
    }
    throw StateError('Unexpected request: ${options.method} $path');
  }
}

class _MemoryPendingStore implements WalletPendingMutationStore {
  final Map<String, Map<String, WalletPendingMutation>> _records =
      <String, Map<String, WalletPendingMutation>>{};
  _AsyncGate? _nextRead;
  _AsyncGate? _nextDelete;

  _AsyncGate pauseNextRead() {
    final _AsyncGate gate = _AsyncGate();
    _nextRead = gate;
    return gate;
  }

  _AsyncGate pauseNextDelete() {
    final _AsyncGate gate = _AsyncGate();
    _nextDelete = gate;
    return gate;
  }

  @override
  Future<void> delete(String accountId, String operationKey) async {
    final _AsyncGate? gate = _nextDelete;
    _nextDelete = null;
    if (gate != null) {
      gate.started.complete();
      await gate.release.future;
    }
    _records[accountId]?.remove(operationKey);
  }

  @override
  Future<List<WalletPendingMutation>> list(String accountId) async {
    return _records[accountId]?.values.toList(growable: false) ??
        <WalletPendingMutation>[];
  }

  @override
  Future<WalletPendingMutation?> read(
    String accountId,
    String operationKey,
  ) async {
    final _AsyncGate? gate = _nextRead;
    _nextRead = null;
    if (gate != null) {
      gate.started.complete();
      await gate.release.future;
    }
    return _records[accountId]?[operationKey];
  }

  @override
  Future<void> write(String accountId, WalletPendingMutation mutation) async {
    (_records[accountId] ??=
            <String, WalletPendingMutation>{})[mutation.operationKey] =
        mutation;
  }
}

class _MemoryWalletSeedStore implements WalletSeedStore {
  final Map<String, List<int>> _seeds = <String, List<int>>{};
  _AsyncGate? _nextRead;

  _AsyncGate pauseNextRead() {
    final _AsyncGate gate = _AsyncGate();
    _nextRead = gate;
    return gate;
  }

  @override
  Future<void> delete(String accountId) async {
    _seeds.remove(accountId);
  }

  @override
  Future<List<int>?> read(String accountId) async {
    final _AsyncGate? gate = _nextRead;
    _nextRead = null;
    if (gate != null) {
      gate.started.complete();
      await gate.release.future;
    }
    final List<int>? seed = _seeds[accountId];
    return seed == null ? null : List<int>.from(seed);
  }

  @override
  Future<void> write(String accountId, List<int> seed) async {
    _seeds[accountId] = List<int>.from(seed);
  }
}

class _AsyncGate {
  final Completer<void> started = Completer<void>();
  final Completer<void> release = Completer<void>();
}
