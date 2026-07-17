import 'dart:async';
import 'dart:convert';

import 'package:cryptography/cryptography.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/config/app_environment.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/core/network/session_interceptor.dart';
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
      expect(harness.signingOutcomeRequests, 1);
      expect(harness.signingOutcomeMethod, 'POST');
      expect(
        harness.signingOutcomePath,
        endsWith('/credit/signing-intent-outcome'),
      );
      expect(harness.signingOutcomePath, isNot(contains(harness.intentId)));
      expect(harness.tipRequests, 1);
      expect(await harness.pendingStore.list('1'), isEmpty);
      expect(harness.tipIdempotencyKey, harness.signingIdempotencyKey);
    });

    test(
      'queries intent outcome when a committed 201 response cannot deserialize',
      () async {
        harness.malformedCommittedTaskResponse = true;

        await expectLater(
          harness.repository.createTask(
            TaskInput(title: 'Malformed response task', rewardAmount: 10),
          ),
          throwsA(isA<WalletMutationCommitted>()),
        );

        expect(harness.taskCreateRequests, 1);
        expect(harness.signingOutcomeRequests, 1);
        expect(await harness.pendingStore.list('1'), isEmpty);
      },
    );

    test(
      'queries intent outcome when a committed task response has null data',
      () async {
        harness.nullCommittedTaskResponse = true;

        await expectLater(
          harness.repository.createTask(
            TaskInput(title: 'Null response task', rewardAmount: 10),
          ),
          throwsA(isA<WalletMutationCommitted>()),
        );

        expect(harness.taskCreateRequests, 1);
        expect(harness.signingOutcomeRequests, 1);
        expect(await harness.pendingStore.list('1'), isEmpty);
      },
    );

    test(
      'queries intent outcome when a committed purchase response has null data',
      () async {
        harness.nullCommittedPurchaseResponse = true;

        await expectLater(
          harness.repository.purchaseProduct(
            Product(
              id: '7',
              sellerId: '2',
              title: 'Product',
              description: null,
              price: 10,
              stock: 1,
              status: ProductStatusEnum.onSale,
              createdAt: 1,
            ),
          ),
          throwsA(isA<WalletMutationCommitted>()),
        );

        expect(harness.purchaseRequests, 1);
        expect(harness.signingOutcomeRequests, 1);
        expect(await harness.pendingStore.list('1'), isEmpty);
      },
    );

    test(
      'persists an unresolved operation and blocks replay after restart',
      () async {
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        final WalletPendingMutation pending = (await harness.pendingStore.list(
          '1',
        )).single;
        expect(pending.operationKey, startsWith('sha256:'));
        expect(pending.operationKey, hasLength(50));
        expect(pending.operationKey, isNot(contains('ledger-tx-1')));

        harness.restoreRepository();
        harness.intentOutcome = _IntentOutcome.committed;
        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationCommitted>()),
        );

        expect(harness.signingIntentRequests, 1);
        expect(harness.tipRequests, 1);
        expect(await harness.pendingStore.list('1'), isEmpty);
      },
    );

    test(
      'retains a backend-pending intent even when the ledger entry is absent',
      () async {
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;
        harness.ledgerContainsTransaction = false;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );

        expect(harness.signingIntentRequests, 1);
        expect(harness.signingOutcomeRequests, 2);
        expect(harness.tipRequests, 1);
        expect(harness.ledgerRequests, 0);
        expect(await harness.pendingStore.list('1'), hasLength(1));
      },
    );

    test('retains a backend-pending intent after a server error', () async {
      harness.tipOutcome = _TipOutcome.serverErrorUnresolved;

      await expectLater(
        harness.repository.tip(_tip()),
        throwsA(isA<WalletMutationUncertain>()),
      );

      expect(harness.signingOutcomeRequests, 1);
      expect(await harness.pendingStore.list('1'), hasLength(1));
    });

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
        expect(await harness.pendingStore.list('1'), isEmpty);
      },
    );

    test(
      'creates a fresh intent only after the backend proves expiry',
      () async {
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        harness.intentOutcome = _IntentOutcome.expired;
        harness.tipOutcome = _TipOutcome.success;

        await harness.repository.tip(_tip());

        expect(harness.signingIntentRequests, 2);
        expect(harness.tipRequests, 2);
        expect(await harness.pendingStore.list('1'), isEmpty);
      },
    );

    test(
      'stale reconciliation cannot delete a replacement pending intent',
      () async {
        final WalletPendingMutation initial = await harness
            .createUnresolvedPending();
        final WalletPendingMutation stale = WalletPendingMutation(
          operationKey: initial.operationKey,
          intentId: '00000000-0000-4000-8000-000000000099',
          expiresAt: initial.expiresAt,
          action: initial.action,
        );
        await harness.pendingStore.write('1', stale);
        harness.restoreRepository();
        harness.intentOutcome = _IntentOutcome.expired;
        harness.tipOutcome = _TipOutcome.lostResponseUnresolved;
        final _AsyncGate staleDelete = harness.pendingStore.pauseNextDelete();

        final Future<WalletSnapshot> load = harness.repository.load();
        await staleDelete.started.future;
        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        expect(
          (await harness.pendingStore.list('1')).single.intentId,
          harness.intentId,
        );

        staleDelete.release.complete();
        expect((await load).pendingMutationCount, 1);
        expect(
          (await harness.pendingStore.list('1')).single.intentId,
          harness.intentId,
        );
        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(isA<WalletMutationUncertain>()),
        );
        expect(harness.signingIntentRequests, 2);
        expect(harness.tipRequests, 2);
      },
    );

    for (final _IntentOutcome unavailableOutcome in <_IntentOutcome>[
      _IntentOutcome.notFound,
      _IntentOutcome.serverError,
    ]) {
      test(
        'retains the pending intent when outcome is ${unavailableOutcome.name}',
        () async {
          harness.tipOutcome = _TipOutcome.lostResponseUnresolved;
          await expectLater(
            harness.repository.tip(_tip()),
            throwsA(isA<WalletMutationUncertain>()),
          );
          harness.restoreRepository();
          harness.intentOutcome = unavailableOutcome;

          await expectLater(
            harness.repository.tip(_tip()),
            throwsA(isA<WalletMutationUncertain>()),
          );

          expect(harness.signingIntentRequests, 1);
          expect(await harness.pendingStore.list('1'), hasLength(1));
        },
      );
    }

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

        expect(await harness.pendingStore.list('1'), isEmpty);
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

    test(
      'rejects self-tip and non-canonical ids before intent creation',
      () async {
        for (final TipInput input in <TipInput>[
          _tip(toAccountId: '1'),
          _tip(toAccountId: '02'),
          _tip(targetId: '010'),
        ]) {
          await expectLater(
            harness.repository.tip(input),
            throwsA(_isFailureKind(ApiFailureKind.invalidInput)),
          );
        }

        expect(harness.walletRequests, 0);
        expect(harness.signingIntentRequests, 0);
        expect(harness.tipRequests, 0);
      },
    );

    test('rejects an insufficient owner balance before signing', () async {
      harness.walletBalance = 0;

      await expectLater(
        harness.repository.tip(_tip()),
        throwsA(_isFailureKind(ApiFailureKind.invalidInput)),
      );

      expect(harness.walletRequests, 1);
      expect(harness.signingIntentRequests, 0);
      expect(harness.tipRequests, 0);
    });

    test(
      'deletes a cancelled task without creating an unused wallet intent',
      () async {
        await harness.repository.updateTask(
          task: Task(
            id: '8',
            creatorId: '1',
            acceptorId: '2',
            title: 'Cancelled task',
            description: null,
            rewardAmount: 30,
            contactInfo: null,
            status: TaskStatusEnum.cancelled,
            createdAt: 1,
          ),
          action: TaskActionActionEnum.delete,
        );

        expect(harness.signingIntentRequests, 0);
        expect(harness.taskActionRequests, 1);
        expect(harness.taskActionIntent, isNull);
        expect(harness.taskActionSignature, isNull);
        expect(harness.taskActionIdempotencyKey, isNull);
      },
    );
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
      expect(harness.session.manager.state.account?.id, '2');
    });

    test(
      'does not query account A pending state after switching during store read',
      () async {
        await harness.createUnresolvedPending();
        harness.restoreRepository();
        final int outcomeRequestsBeforeRetry = harness.signingOutcomeRequests;
        final _AsyncGate pendingRead = harness.pendingStore.pauseNextRead();

        final Future<void> retry = harness.repository.tip(_tip());
        await pendingRead.started.future;
        await harness.switchToAccount2();
        pendingRead.release.complete();

        await expectLater(retry, throwsA(_isCancelledFailure));
        expect(harness.signingOutcomeRequests, outcomeRequestsBeforeRetry);
        expect(await harness.pendingStore.list('1'), hasLength(1));
      },
    );

    test(
      'discards account A reconciliation result when the session switches in flight',
      () async {
        await harness.createUnresolvedPending();
        harness.restoreRepository();
        harness.intentOutcome = _IntentOutcome.committed;
        final _AsyncGate outcomeQuery = harness.pauseNextSigningOutcomeQuery();

        final Future<void> retry = harness.repository.tip(_tip());
        await outcomeQuery.started.future;
        await harness.switchToAccount2();
        outcomeQuery.release.complete();

        await expectLater(retry, throwsA(_isCancelledFailure));
        expect(await harness.pendingStore.list('1'), hasLength(1));
        expect(harness.signingIntentRequests, 1);
      },
    );

    test(
      'restores account A anti-replay record when the session switches during delete',
      () async {
        await harness.createUnresolvedPending();
        harness.restoreRepository();
        harness.intentOutcome = _IntentOutcome.expired;
        final _AsyncGate pendingDelete = harness.pendingStore.pauseNextDelete();

        final Future<void> retry = harness.repository.tip(_tip());
        await pendingDelete.started.future;
        await harness.switchToAccount2();
        pendingDelete.release.complete();

        await expectLater(retry, throwsA(_isCancelledFailure));
        expect(await harness.pendingStore.list('1'), hasLength(1));
        expect(harness.signingIntentRequests, 1);
      },
    );
  });

  group('WalletRepository active public key boundary', () {
    late _WalletHarness harness;

    setUp(() async {
      harness = _WalletHarness();
      await harness.initialize();
    });

    tearDown(() => harness.dispose());

    test(
      'fails before intent creation when activePublicKey is omitted',
      () async {
        harness.includeActivePublicKey = false;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(_isFailureKind(ApiFailureKind.unexpected)),
        );

        expect(harness.walletRequests, 1);
        expect(harness.signingIntentRequests, 0);
        expect(harness.tipRequests, 0);
      },
    );

    test('fails before intent creation when activePublicKey is null', () async {
      harness.walletActivePublicKey = null;

      await expectLater(
        harness.repository.tip(_tip()),
        throwsA(_isFailureKind(ApiFailureKind.invalidInput)),
      );

      expect(harness.walletRequests, 1);
      expect(harness.signingIntentRequests, 0);
    });

    test(
      'fails before intent creation when activePublicKey is not canonical',
      () async {
        harness.walletActivePublicKey = 'not-canonical-base64';

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(_isFailureKind(ApiFailureKind.unexpected)),
        );

        expect(harness.walletRequests, 1);
        expect(harness.signingIntentRequests, 0);
      },
    );

    test(
      'fails before intent creation when the local key does not match',
      () async {
        final LocalWalletKey otherKey = await harness.signer.generate(
          'other-account',
        );
        harness.walletActivePublicKey = otherKey.publicKeyBase64;

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(_isFailureKind(ApiFailureKind.conflict)),
        );

        expect(harness.walletRequests, 1);
        expect(harness.signingIntentRequests, 0);
        expect(harness.tipRequests, 0);
      },
    );

    test(
      'fails before intent creation when the wallet account does not match',
      () async {
        harness.walletResponseAccountId = '2';

        await expectLater(
          harness.repository.tip(_tip()),
          throwsA(_isFailureKind(ApiFailureKind.unexpected)),
        );

        expect(harness.walletRequests, 1);
        expect(harness.signingIntentRequests, 0);
      },
    );

    test('creates the signing intent only after an exact key match', () async {
      await harness.repository.tip(_tip());

      expect(harness.walletRequests, 1);
      expect(harness.signingIntentRequests, 1);
      expect(harness.tipRequests, 1);
    });

    for (final testCase
        in <
          ({String label, void Function(Map<String, Object?> envelope) mutate})
        >[
          (
            label: 'version',
            mutate: (Map<String, Object?> envelope) => envelope['version'] = 2,
          ),
          (
            label: 'accountId',
            mutate: (Map<String, Object?> envelope) =>
                envelope['accountId'] = '2',
          ),
          (
            label: 'action',
            mutate: (Map<String, Object?> envelope) =>
                envelope['action'] = 'credit.task.create',
          ),
          (
            label: 'expiresAt',
            mutate: (Map<String, Object?> envelope) =>
                envelope['expiresAt'] = 1999999999,
          ),
          (
            label: 'idempotencyKey',
            mutate: (Map<String, Object?> envelope) =>
                envelope['idempotencyKey'] = 'credit:other',
          ),
          (
            label: 'intentId',
            mutate: (Map<String, Object?> envelope) =>
                envelope['intentId'] = '00000000-0000-4000-8000-000000000002',
          ),
          (
            label: 'publicKey',
            mutate: (Map<String, Object?> envelope) =>
                envelope['publicKey'] = 'other-public-key',
          ),
          (
            label: 'requestHash',
            mutate: (Map<String, Object?> envelope) =>
                envelope['requestHash'] = '00',
          ),
          (
            label: 'owner balance snapshot',
            mutate: (Map<String, Object?> envelope) =>
                envelope['snapshot'] = <String, Object>{'balance': 99},
          ),
          (
            label: 'ledger amount',
            mutate: (Map<String, Object?> envelope) {
              (envelope['ledgerEntry']! as Map<String, Object?>)['amount'] = 2;
            },
          ),
          (
            label: 'ledger recipient',
            mutate: (Map<String, Object?> envelope) {
              (envelope['ledgerEntry']! as Map<String, Object?>)['to_account'] =
                  '3';
            },
          ),
          (
            label: 'ledger metadata',
            mutate: (Map<String, Object?> envelope) {
              final Map<String, Object?> ledger =
                  envelope['ledgerEntry']! as Map<String, Object?>;
              (ledger['metadata']! as Map<String, Object?>)['target_id'] = '11';
            },
          ),
          (
            label: 'extra metadata field',
            mutate: (Map<String, Object?> envelope) {
              final Map<String, Object?> ledger =
                  envelope['ledgerEntry']! as Map<String, Object?>;
              (ledger['metadata']! as Map<String, Object?>)['note'] = 'hidden';
            },
          ),
          (
            label: 'extra envelope field',
            mutate: (Map<String, Object?> envelope) =>
                envelope['hidden'] = true,
          ),
          (
            label: 'missing envelope field',
            mutate: (Map<String, Object?> envelope) =>
                envelope.remove('ledgerEntry'),
          ),
          (
            label: 'ledger transaction UUID',
            mutate: (Map<String, Object?> envelope) {
              (envelope['ledgerEntry']! as Map<String, Object?>)['tx_id'] =
                  'not-a-uuid';
            },
          ),
        ]) {
      test(
        'does not persist or execute an intent with mismatched ${testCase.label}',
        () async {
          harness.signingEnvelopeMutation = testCase.mutate;

          await expectLater(
            harness.repository.tip(_tip()),
            throwsA(_isFailureKind(ApiFailureKind.unexpected)),
          );

          expect(harness.signingIntentRequests, 1);
          expect(harness.tipRequests, 0);
          expect(await harness.pendingStore.list('1'), isEmpty);
        },
      );
    }

    test('does not sign a non-canonical envelope', () async {
      harness.signingBytesMutation = (String signingBytes) => ' $signingBytes';

      await expectLater(
        harness.repository.tip(_tip()),
        throwsA(_isFailureKind(ApiFailureKind.unexpected)),
      );

      expect(harness.tipRequests, 0);
      expect(await harness.pendingStore.list('1'), isEmpty);
    });

    test(
      'generates a key only when the owner wallet is explicitly unbound',
      () async {
        await harness.signer.delete('1');
        harness.walletActivePublicKey = null;

        final LocalWalletKey key = await harness.repository
            .createAndBindLocalKey();

        expect(key.publicKeyBase64, isNotEmpty);
        expect(harness.bindRequests, 1);
        expect(
          (await harness.signer.readPublicKey('1'))?.publicKeyBase64,
          key.publicKeyBase64,
        );
      },
    );

    test(
      'serializes concurrent first binding around one persisted account key',
      () async {
        await harness.signer.delete('1');
        harness.walletActivePublicKey = null;
        final int writesBeforeBinding = harness.seedStore.persistedWrites;

        final List<LocalWalletKey> keys =
            await Future.wait(<Future<LocalWalletKey>>[
              harness.repository.createAndBindLocalKey(),
              harness.repository.createAndBindLocalKey(),
            ]);

        expect(
          keys.map((LocalWalletKey key) => key.publicKeyBase64).toSet(),
          hasLength(1),
        );
        expect(harness.bindRequests, 2);
        expect(harness.bindPublicKeys.toSet(), <String>{
          keys.first.publicKeyBase64,
        });
        expect(harness.walletActivePublicKey, keys.first.publicKeyBase64);
        expect(
          (await harness.signer.readPublicKey('1'))?.publicKeyBase64,
          harness.walletActivePublicKey,
        );
        expect(harness.seedStore.persistedWrites, writesBeforeBinding + 1);
      },
    );

    test('does not generate when the owner key field is missing', () async {
      await harness.signer.delete('1');
      harness.includeActivePublicKey = false;

      await expectLater(
        harness.repository.createAndBindLocalKey(),
        throwsA(_isFailureKind(ApiFailureKind.unexpected)),
      );

      expect(await harness.signer.readPublicKey('1'), isNull);
      expect(harness.bindRequests, 0);
    });

    test(
      'does not generate a replacement when the bound key is missing locally',
      () async {
        final String boundPublicKey = harness.walletActivePublicKey!;
        await harness.signer.delete('1');

        await expectLater(
          harness.repository.createAndBindLocalKey(),
          throwsA(_isFailureKind(ApiFailureKind.unexpected)),
        );

        expect(harness.walletActivePublicKey, boundPublicKey);
        expect(await harness.signer.readPublicKey('1'), isNull);
        expect(harness.bindRequests, 0);
      },
    );

    test(
      'does not bind a local key that differs from the bound owner key',
      () async {
        harness.walletActivePublicKey = base64Encode(
          List<int>.filled(32, 0x41),
        );

        await expectLater(
          harness.repository.createAndBindLocalKey(),
          throwsA(_isFailureKind(ApiFailureKind.conflict)),
        );

        expect(harness.bindRequests, 0);
      },
    );

    test('allows an exact existing key to be rebound idempotently', () async {
      final LocalWalletKey key = await harness.repository
          .createAndBindLocalKey();

      expect(key.publicKeyBase64, harness.walletActivePublicKey);
      expect(harness.bindRequests, 1);
    });

    test(
      'keeps bind body and credentials on account A when B becomes active before dispatch',
      () async {
        final int accountAGeneration = harness.session.manager.generation;
        final _AsyncGate dispatch = harness.pauseBindBeforeSessionDispatch();

        final Future<LocalWalletKey> binding = harness.repository
            .createAndBindLocalKey();
        await dispatch.started.future;
        await harness.switchToAccount2();
        dispatch.release.complete();

        await expectLater(binding, throwsA(_isCancelledFailure));
        expect(harness.bindRequests, 1);
        expect(harness.bindBodyAccountId, '1');
        expect(harness.bindAuthorization, 'Bearer access-new');
        expect(harness.bindSessionAccountId, '1');
        expect(harness.bindSessionGeneration, accountAGeneration);
        expect(harness.bindAuthorization, isNot('Bearer access-account-2'));
      },
    );

    test(
      'stops bind when the bearer rotates within the captured generation',
      () async {
        final int generation = harness.session.manager.generation;
        harness.refreshAccessToken = 'access-rotated';
        harness.rejectNextWalletAccessToken = true;
        harness.enableSessionInterceptor();

        await expectLater(
          harness.repository.createAndBindLocalKey(),
          throwsA(_isCancelledFailure),
        );

        expect(harness.session.manager.generation, generation);
        expect(harness.session.manager.accessToken, 'access-rotated');
        expect(harness.bindRequests, 0);
      },
    );

    test(
      'stops account A before intent creation when the session switches',
      () async {
        final _AsyncGate walletQuery = harness.pauseNextWalletQuery();

        final Future<void> tip = harness.repository.tip(_tip());
        await walletQuery.started.future;
        await harness.switchToAccount2();
        walletQuery.release.complete();

        await expectLater(tip, throwsA(_isCancelledFailure));
        expect(harness.walletRequests, 1);
        expect(harness.signingIntentRequests, 0);
        expect(harness.tipRequests, 0);
      },
    );
  });
}

final Matcher _isCancelledFailure = isA<ApiFailure>().having(
  (ApiFailure failure) => failure.kind,
  'kind',
  ApiFailureKind.cancelled,
);

Matcher _isFailureKind(ApiFailureKind kind) => isA<ApiFailure>().having(
  (ApiFailure failure) => failure.kind,
  'kind',
  kind,
);

Future<Object> _captureOutcome(Future<void> operation) async {
  try {
    await operation;
    return 'success';
  } on Object catch (error) {
    return error;
  }
}

TipInput _tip({
  int amount = 1,
  String toAccountId = '2',
  String targetId = '10',
}) => TipInput(
  toAccountId: toAccountId,
  amount: amount,
  targetType: TipInputTargetTypeEnum.thread,
  targetId: targetId,
);

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

enum _TipOutcome {
  success,
  rejected,
  serverErrorUnresolved,
  lostResponseCommitted,
  lostResponseUnresolved,
}

enum _IntentOutcome { pending, committed, expired, notFound, serverError }

class _WalletHarness {
  late SessionHarness session;
  final _MemoryWalletSeedStore seedStore = _MemoryWalletSeedStore();
  final _MemoryPendingStore pendingStore = _MemoryPendingStore();
  late WalletSigner signer;
  late WalletRepository repository;
  late YourtjApi api;
  _TipOutcome tipOutcome = _TipOutcome.success;
  _IntentOutcome intentOutcome = _IntentOutcome.pending;
  bool ledgerContainsTransaction = false;
  bool malformedCommittedTaskResponse = false;
  bool nullCommittedTaskResponse = false;
  bool nullCommittedPurchaseResponse = false;
  int signingIntentRequests = 0;
  int signingOutcomeRequests = 0;
  int tipRequests = 0;
  int taskCreateRequests = 0;
  int purchaseRequests = 0;
  int taskActionRequests = 0;
  int bindRequests = 0;
  final List<String> bindPublicKeys = <String>[];
  int ledgerRequests = 0;
  int walletRequests = 0;
  int walletBalance = 100;
  bool includeActivePublicKey = true;
  bool rejectNextWalletAccessToken = false;
  String walletResponseAccountId = '1';
  String refreshAccessToken = 'access-new';
  String? walletActivePublicKey;
  String? signingIdempotencyKey;
  String? tipIdempotencyKey;
  String? taskActionIntent;
  String? taskActionSignature;
  String? taskActionIdempotencyKey;
  String? bindBodyAccountId;
  String? bindAuthorization;
  String? bindSessionAccountId;
  int? bindSessionGeneration;
  String? signingOutcomeMethod;
  String? signingOutcomePath;
  void Function(Map<String, Object?> envelope)? signingEnvelopeMutation;
  String Function(String signingBytes)? signingBytesMutation;
  _AsyncGate? _nextSigningOutcomeQuery;
  _AsyncGate? _nextWalletQuery;

  Future<void> initialize() async {
    session = SessionHarness(
      storage: MemorySessionStorage(
        activeAccountId: '1',
        refreshTokens: <String, String>{'1': 'refresh-old'},
      ),
      handler: _handle,
    );
    await session.manager.initialize();
    api = YourtjApi(dio: session.dio, interceptors: const []);
    signer = WalletSigner(seedStore);
    final LocalWalletKey localKey = await signer.generate('1');
    walletActivePublicKey = localKey.publicKeyBase64;
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

  Future<WalletPendingMutation> createUnresolvedPending() async {
    tipOutcome = _TipOutcome.lostResponseUnresolved;
    await expectLater(
      repository.tip(_tip()),
      throwsA(isA<WalletMutationUncertain>()),
    );
    return (await pendingStore.list('1')).single;
  }

  _AsyncGate pauseNextSigningOutcomeQuery() {
    final _AsyncGate gate = _AsyncGate();
    _nextSigningOutcomeQuery = gate;
    return gate;
  }

  _AsyncGate pauseNextWalletQuery() {
    final _AsyncGate gate = _AsyncGate();
    _nextWalletQuery = gate;
    return gate;
  }

  _AsyncGate pauseBindBeforeSessionDispatch() {
    final _AsyncGate gate = _AsyncGate();
    final AppEnvironment environment = AppEnvironment(
      apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
    );
    session.dio.interceptors.add(_BindDispatchGateInterceptor(gate));
    enableSessionInterceptor(environment: environment);
    return gate;
  }

  void enableSessionInterceptor({AppEnvironment? environment}) {
    final AppEnvironment resolvedEnvironment =
        environment ??
        AppEnvironment(apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'));
    session.dio.interceptors.add(
      SessionInterceptor(session.dio, resolvedEnvironment, session.manager),
    );
  }

  Future<void> dispose() => session.dispose();

  Future<ResponseBody> _handle(RequestOptions options) async {
    final String path = options.uri.path;
    if (path.endsWith('/auth/refresh')) {
      return jsonResponse(
        authTokensJson(
          accountId: '1',
          accessToken: refreshAccessToken,
          refreshToken: 'refresh-new',
        ),
      );
    }
    if (path.endsWith('/auth/password/login')) {
      return jsonResponse(
        authTokensJson(
          accountId: '2',
          accessToken: 'access-account-2',
          refreshToken: 'refresh-account-2',
        ),
      );
    }
    if (path.endsWith('/wallet/bind')) {
      bindRequests += 1;
      final Map<String, dynamic> body = Map<String, dynamic>.from(
        jsonDecode(options.data! as String) as Map,
      );
      bindBodyAccountId = body['accountId'] as String?;
      final String publicKey = body['publicKey']! as String;
      bindPublicKeys.add(publicKey);
      bindAuthorization = _header(options.headers, 'authorization');
      bindSessionAccountId =
          options.extra['yourtj.sessionAccountId'] as String?;
      bindSessionGeneration = options.extra['yourtj.sessionGeneration'] as int?;
      final String? activePublicKey = walletActivePublicKey;
      if (activePublicKey != null && activePublicKey != publicKey) {
        return jsonResponse(<String, Object>{
          'error': <String, String>{
            'code': 'WALLET_KEY_ALREADY_BOUND',
            'message': '钱包已绑定其他公钥',
          },
        }, statusCode: 409);
      }
      walletActivePublicKey = publicKey;
      return ResponseBody.fromString('', 204);
    }
    if (path.endsWith('/wallet')) {
      walletRequests += 1;
      if (rejectNextWalletAccessToken &&
          _header(options.headers, 'authorization') == 'Bearer access-new') {
        rejectNextWalletAccessToken = false;
        return jsonResponse(<String, Object>{
          'error': <String, String>{
            'code': 'UNAUTHORIZED',
            'message': 'expired',
          },
        }, statusCode: 401);
      }
      final _AsyncGate? gate = _nextWalletQuery;
      _nextWalletQuery = null;
      if (gate != null) {
        gate.started.complete();
        await gate.release.future;
      }
      final Map<String, Object?> body = <String, Object?>{
        'accountId': walletResponseAccountId,
        'balance': walletBalance,
      };
      if (includeActivePublicKey) {
        body['activePublicKey'] = walletActivePublicKey;
      }
      return jsonResponse(body);
    }
    if (path.endsWith('/wallet/ledger/verify')) {
      return jsonResponse(<String, Object>{
        'ok': true,
        'latestSeq': 41,
        'latestHash': 'hash-41',
      });
    }
    if (path.endsWith('/credit/signing-intent-outcome')) {
      signingOutcomeRequests += 1;
      signingOutcomeMethod = options.method;
      signingOutcomePath = path;
      final _AsyncGate? gate = _nextSigningOutcomeQuery;
      _nextSigningOutcomeQuery = null;
      if (gate != null) {
        gate.started.complete();
        await gate.release.future;
      }
      final Map<String, dynamic> outcomeInput = Map<String, dynamic>.from(
        jsonDecode(options.data! as String) as Map,
      );
      final String intentId = outcomeInput['intentId']! as String;
      switch (intentOutcome) {
        case _IntentOutcome.pending:
        case _IntentOutcome.committed:
        case _IntentOutcome.expired:
          return jsonResponse(<String, Object>{
            'intentId': intentId,
            'status': intentOutcome.name,
            'expiresAt': 2000000000,
          });
        case _IntentOutcome.notFound:
          return jsonResponse(<String, Object>{
            'error': <String, String>{
              'code': 'NOT_FOUND',
              'message': '签名请求不存在',
            },
          }, statusCode: 404);
        case _IntentOutcome.serverError:
          return jsonResponse(<String, Object>{
            'error': <String, String>{
              'code': 'INTERNAL',
              'message': '暂时无法查询签名请求',
            },
          }, statusCode: 500);
      }
    }
    if (path.endsWith('/credit/signing-intents')) {
      signingIntentRequests += 1;
      intentOutcome = _IntentOutcome.pending;
      signingIdempotencyKey = options.headers['Idempotency-Key'] as String?;
      final Map<String, dynamic> signingInput = Map<String, dynamic>.from(
        jsonDecode(options.data! as String) as Map,
      );
      final Map<String, Object?> request = Map<String, Object?>.from(
        signingInput['request']! as Map,
      );
      final String signingAction = signingInput['action']! as String;
      late final Map<String, Object?> snapshot;
      Map<String, Object?> requestForHash = request;
      Object? ledgerEntry;
      if (signingAction == 'credit.tip') {
        snapshot = <String, Object>{'balance': walletBalance};
        ledgerEntry = <String, Object?>{
          'tx_id': '00000000-0000-4000-8000-000000000002',
          'type': 'tip',
          'from_account': '1',
          'to_account': request['toAccountId'],
          'amount': request['amount'],
          'nonce': '00000000-0000-4000-8000-000000000003',
          'metadata': <String, Object?>{
            'target_type': request['targetType'],
            'target_id': request['targetId'],
            'signing_intent_id': '00000000-0000-4000-8000-000000000001',
          },
          'signer': '1',
          'timestamp': 1999999700,
        };
      } else if (signingAction == 'credit.task.create') {
        requestForHash = <String, Object?>{
          'title': request['title'],
          'description': request['description'],
          'rewardAmount': request['rewardAmount'],
          'contactInfo': request['contactInfo'],
        };
        snapshot = <String, Object>{'balance': walletBalance};
        ledgerEntry = <String, Object?>{
          'tx_id': '00000000-0000-4000-8000-000000000002',
          'type': 'escrow_hold',
          'from_account': '1',
          'to_account': null,
          'amount': request['rewardAmount'],
          'nonce': '00000000-0000-4000-8000-000000000003',
          'metadata': <String, Object?>{
            'signing_intent_id': '00000000-0000-4000-8000-000000000001',
          },
          'signer': '1',
          'timestamp': 1999999700,
        };
      } else if (signingAction == 'credit.task.action') {
        snapshot = <String, Object?>{
          'status': 'cancelled',
          'partyA': '1',
          'partyB': '2',
          'amount': 30,
          'actorId': '1',
        };
      } else if (signingAction == 'credit.product.purchase') {
        snapshot = <String, Object?>{
          'price': 10,
          'stock': 1,
          'status': 'on_sale',
          'sellerId': '2',
        };
        ledgerEntry = <String, Object?>{
          'tx_id': '00000000-0000-4000-8000-000000000002',
          'type': 'escrow_hold',
          'from_account': '1',
          'to_account': null,
          'amount': 10,
          'nonce': '00000000-0000-4000-8000-000000000003',
          'metadata': <String, Object?>{
            'product_id': request['productId'],
            'signing_intent_id': '00000000-0000-4000-8000-000000000001',
          },
          'signer': '1',
          'timestamp': 1999999700,
        };
      } else {
        throw StateError('Unexpected signing action: $signingAction');
      }
      final Map<String, Object?> signingEnvelope = <String, Object?>{
        'version': 1,
        'intentId': '00000000-0000-4000-8000-000000000001',
        'accountId': '1',
        'publicKey': walletActivePublicKey,
        'action': signingAction,
        'requestHash': await _requestHash(requestForHash),
        'snapshot': snapshot,
        'idempotencyKey': signingIdempotencyKey,
        'expiresAt': 2000000000,
        'ledgerEntry': ledgerEntry,
      };
      signingEnvelopeMutation?.call(signingEnvelope);
      final String signingBytes =
          signingBytesMutation?.call(_canonicalJson(signingEnvelope)) ??
          _canonicalJson(signingEnvelope);
      return jsonResponse(<String, Object>{
        'intentId': '00000000-0000-4000-8000-000000000001',
        'signingBytes': signingBytes,
        'expiresAt': 2000000000,
      });
    }
    if (options.method == 'GET' &&
        (path.endsWith('/credit/tasks') ||
            path.endsWith('/credit/products') ||
            path.endsWith('/credit/purchases'))) {
      return jsonResponse(<String, Object?>{
        'items': <Object>[],
        'nextCursor': null,
        'hasMore': false,
      });
    }
    if (path.endsWith('/credit/tasks') && malformedCommittedTaskResponse) {
      taskCreateRequests += 1;
      intentOutcome = _IntentOutcome.committed;
      return jsonResponse('malformed committed task response', statusCode: 201);
    }
    if (path.endsWith('/credit/tasks') && nullCommittedTaskResponse) {
      taskCreateRequests += 1;
      intentOutcome = _IntentOutcome.committed;
      return jsonResponse(null, statusCode: 201);
    }
    if (path.endsWith('/credit/products/7/purchase') &&
        nullCommittedPurchaseResponse) {
      purchaseRequests += 1;
      intentOutcome = _IntentOutcome.committed;
      return jsonResponse(null, statusCode: 201);
    }
    if (path.endsWith('/credit/tip')) {
      tipRequests += 1;
      tipIdempotencyKey = options.headers['Idempotency-Key'] as String?;
      switch (tipOutcome) {
        case _TipOutcome.success:
          ledgerContainsTransaction = true;
          intentOutcome = _IntentOutcome.committed;
          return ResponseBody.fromString('', 204);
        case _TipOutcome.rejected:
          return jsonResponse(<String, Object>{
            'error': <String, String>{
              'code': 'INVALID_TARGET',
              'message': '目标无效',
            },
          }, statusCode: 400);
        case _TipOutcome.serverErrorUnresolved:
          return jsonResponse(<String, Object>{
            'error': <String, String>{
              'code': 'INTERNAL',
              'message': '暂时无法完成打赏',
            },
          }, statusCode: 500);
        case _TipOutcome.lostResponseCommitted:
          ledgerContainsTransaction = true;
          intentOutcome = _IntentOutcome.committed;
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
    if (path.endsWith('/credit/tasks/8/action')) {
      taskActionRequests += 1;
      taskActionIntent = options.headers['X-Wallet-Intent'] as String?;
      taskActionSignature = options.headers['X-Wallet-Sig'] as String?;
      taskActionIdempotencyKey = options.headers['Idempotency-Key'] as String?;
      return ResponseBody.fromString('', 204);
    }
    if (path.endsWith('/wallet/ledger')) {
      ledgerRequests += 1;
      return jsonResponse(<String, Object?>{
        'items': ledgerContainsTransaction
            ? <Object>[
                <String, Object>{
                  'seq': 42,
                  'txId': 'ledger-tx-1',
                  'type': 'tip',
                  'fromAccount': '1',
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

  String get intentId => '00000000-0000-4000-8000-000000000001';
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
  Future<bool> delete(
    String accountId,
    WalletPendingMutation expectedMutation,
  ) async {
    final _AsyncGate? gate = _nextDelete;
    _nextDelete = null;
    if (gate != null) {
      gate.started.complete();
      await gate.release.future;
    }
    final WalletPendingMutation? current =
        _records[accountId]?[expectedMutation.operationKey];
    if (current == null || !_sameMutation(current, expectedMutation)) {
      return false;
    }
    _records[accountId]?.remove(expectedMutation.operationKey);
    return true;
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

  bool _sameMutation(
    WalletPendingMutation current,
    WalletPendingMutation expected,
  ) {
    return current.operationKey == expected.operationKey &&
        current.intentId == expected.intentId &&
        current.expiresAt == expected.expiresAt &&
        current.action == expected.action;
  }
}

class _MemoryWalletSeedStore implements WalletSeedStore {
  final Map<String, List<int>> _seeds = <String, List<int>>{};
  _AsyncGate? _nextRead;
  int persistedWrites = 0;

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
  Future<List<int>> writeIfAbsent(String accountId, List<int> seed) async {
    final List<int>? existing = _seeds[accountId];
    if (existing != null) {
      return List<int>.from(existing);
    }
    final List<int> persisted = List<int>.from(seed);
    _seeds[accountId] = persisted;
    persistedWrites += 1;
    return List<int>.from(persisted);
  }
}

class _AsyncGate {
  final Completer<void> started = Completer<void>();
  final Completer<void> release = Completer<void>();
}

class _BindDispatchGateInterceptor extends Interceptor {
  _BindDispatchGateInterceptor(this._gate);

  final _AsyncGate _gate;

  @override
  void onRequest(RequestOptions options, RequestInterceptorHandler handler) {
    if (!options.uri.path.endsWith('/wallet/bind')) {
      handler.next(options);
      return;
    }
    _gate.started.complete();
    unawaited(
      _gate.release.future.then<void>((void _) {
        handler.next(options);
      }),
    );
  }
}

String? _header(Map<String, dynamic> headers, String name) {
  for (final MapEntry<String, dynamic> header in headers.entries) {
    if (header.key.toLowerCase() == name.toLowerCase()) {
      return header.value?.toString();
    }
  }
  return null;
}
