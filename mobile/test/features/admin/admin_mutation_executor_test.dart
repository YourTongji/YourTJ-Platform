import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/admin/data/admin_mutation_executor.dart';
import 'package:yourtj_mobile/features/admin/data/admin_repository.dart';
import 'package:yourtj_mobile/features/admin/domain/admin_capabilities.dart';
import 'package:yourtj_mobile/features/admin/domain/admin_mutations.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test('missing capability rejects mutation before touching network', () async {
    final _MutationHarness harness = _MutationHarness(
      (RequestOptions options) => jsonResponse(null),
    );

    await expectLater(
      harness.executor.execute(
        _action(
          AdminMutationKind.triggerSelectionSync,
          AdminCapabilities.runOperations,
        ),
        _submission(),
        _actor(const <String>[]),
      ),
      throwsA(isA<AdminAccessDenied>()),
    );
    expect(harness.adapter.requests, isEmpty);
    harness.dispose();
  });

  test(
    'self and hierarchy checks reject user mutation before network',
    () async {
      final _MutationHarness harness = _MutationHarness(
        (RequestOptions options) => jsonResponse(null),
      );
      final AdminMutationAction action = AdminMutationAction(
        kind: AdminMutationKind.changeUserRole,
        label: '变更角色',
        impact: '角色会变化',
        requiredAnyCapability: const <String>{AdminCapabilities.changeRoles},
        targetId: 'actor-id',
        targetAccountId: 'actor-id',
        targetRole: 'admin',
      );

      await expectLater(
        harness.executor.execute(
          action,
          _submission(<String, String>{'role': 'user'}),
          _actor(const <String>[AdminCapabilities.changeRoles]),
        ),
        throwsA(isA<AdminAccessDenied>()),
      );
      await expectLater(
        harness.executor.execute(
          AdminMutationAction(
            kind: AdminMutationKind.changeUserRole,
            label: '变更角色',
            impact: '角色会变化',
            requiredAnyCapability: const <String>{
              AdminCapabilities.changeRoles,
            },
            targetId: 'another-admin',
            targetAccountId: 'another-admin',
            targetRole: 'admin',
          ),
          _submission(<String, String>{'role': 'user'}),
          _actor(const <String>[AdminCapabilities.changeRoles]),
        ),
        throwsA(isA<AdminAccessDenied>()),
      );
      expect(harness.adapter.requests, isEmpty);
      harness.dispose();
    },
  );

  test(
    'appeal decision sends reviewed version, outcome, and reason once',
    () async {
      final _MutationHarness harness = _MutationHarness(
        (RequestOptions options) => jsonResponse(null),
      );
      const String reason = '复核完整证据后确认原决定应当撤销';

      await harness.executor.execute(
        AdminMutationAction(
          kind: AdminMutationKind.decideAppeal,
          label: '裁决申诉',
          impact: '改变原治理决定',
          requiredAnyCapability: const <String>{
            AdminCapabilities.reviewAppeals,
          },
          targetId: 'appeal-1',
          expectedVersion: 7,
        ),
        _submission(<String, String>{'outcome': 'overturned'}, reason),
        _actor(const <String>[AdminCapabilities.reviewAppeals]),
      );

      expect(harness.adapter.requests, hasLength(1));
      final RecordedRequest request = harness.adapter.requests.single;
      expect(request.method, 'POST');
      expect(request.uri.path, '/api/v2/admin/appeals/appeal-1/decision');
      expect(requestJson(request), <String, Object>{
        'expectedVersion': 7,
        'outcome': 'overturned',
        'reason': reason,
      });
      harness.dispose();
    },
  );

  test('HTTP 409 is exposed as conflict and is never retried', () async {
    final _MutationHarness harness = _MutationHarness(
      (RequestOptions options) => jsonResponse(<String, Object>{
        'error': <String, String>{
          'code': 'VERSION_CONFLICT',
          'message': 'version changed',
        },
      }, statusCode: 409),
    );

    await expectLater(
      harness.executor.execute(
        AdminMutationAction(
          kind: AdminMutationKind.startAppealReview,
          label: '开始处理',
          impact: '领取申诉',
          requiredAnyCapability: const <String>{
            AdminCapabilities.reviewAppeals,
          },
          targetId: 'appeal-1',
          expectedVersion: 3,
        ),
        _submission(),
        _actor(const <String>[AdminCapabilities.reviewAppeals]),
      ),
      throwsA(
        isA<ApiFailure>().having(
          (ApiFailure failure) => failure.kind,
          'kind',
          ApiFailureKind.conflict,
        ),
      ),
    );
    expect(harness.adapter.requests, hasLength(1));
    harness.dispose();
  });

  test('HTTP 428 requires a new confirmation and is never retried', () async {
    final _MutationHarness harness = _MutationHarness(
      (RequestOptions options) => jsonResponse(<String, Object>{
        'error': <String, String>{
          'code': 'RECENT_AUTH_REQUIRED',
          'message': 'authenticate again',
        },
      }, statusCode: 428),
    );

    await expectLater(
      harness.executor.execute(
        _action(
          AdminMutationKind.triggerSelectionSync,
          AdminCapabilities.runOperations,
        ),
        _submission(),
        _actor(const <String>[AdminCapabilities.runOperations]),
      ),
      throwsA(isA<AdminRecentAuthenticationRequired>()),
    );
    expect(harness.adapter.requests, hasLength(1));
    harness.dispose();
  });

  test('credit reconciliation sends one fresh idempotency key', () async {
    final _MutationHarness harness = _MutationHarness(
      (RequestOptions options) => jsonResponse(null),
    );

    await harness.executor.execute(
      _action(
        AdminMutationKind.startCreditReconciliation,
        AdminCapabilities.manageCreditIntegrity,
      ),
      _submission(),
      _actor(const <String>[AdminCapabilities.manageCreditIntegrity]),
    );

    expect(harness.adapter.requests, hasLength(1));
    final RecordedRequest request = harness.adapter.requests.single;
    expect(request.uri.path, '/api/v2/admin/credit/reconciliations');
    final MapEntry<String, dynamic> header = request.headers.entries.firstWhere(
      (MapEntry<String, dynamic> entry) =>
          entry.key.toLowerCase() == 'idempotency-key',
    );
    expect(
      header.value,
      matches(
        RegExp(
          r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
        ),
      ),
    );
    harness.dispose();
  });

  test('self-review media exception must be explicitly confirmed', () async {
    final _MutationHarness harness = _MutationHarness(
      (RequestOptions options) => jsonResponse(null),
    );

    await expectLater(
      harness.executor.execute(
        AdminMutationAction(
          kind: AdminMutationKind.approveMedia,
          label: '批准媒体',
          impact: '发布媒体',
          requiredAnyCapability: const <String>{
            AdminCapabilities.moderateContent,
          },
          targetId: 'upload-1',
        ),
        _submission(<String, String>{'selfReviewConfirmed': 'false'}),
        _actor(const <String>[AdminCapabilities.moderateContent]),
      ),
      throwsA(
        isA<AdminMutationValidation>().having(
          (AdminMutationValidation error) => error.message,
          'message',
          contains('明确确认'),
        ),
      ),
    );
    expect(harness.adapter.requests, isEmpty);
    harness.dispose();
  });
}

class _MutationHarness {
  _MutationHarness(AdapterHandler handler)
    : dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2')),
      adapter = RecordingAdapter(handler) {
    dio.httpClientAdapter = adapter;
    executor = AdminMutationExecutor(AdminApi(dio));
  }

  final Dio dio;
  final RecordingAdapter adapter;
  late final AdminMutationExecutor executor;

  void dispose() => dio.close(force: true);
}

AdminMutationAction _action(AdminMutationKind kind, String capability) =>
    AdminMutationAction(
      kind: kind,
      label: kind.name,
      impact: '测试影响',
      requiredAnyCapability: <String>{capability},
    );

AdminMutationSubmission _submission([
  Map<String, String> values = const <String, String>{},
  String reason = '这是足够详细的管理操作理由',
]) => AdminMutationSubmission(reason: reason, values: values);

AdminActorContext _actor(Iterable<String> capabilities) => AdminActorContext(
  accountId: 'actor-id',
  role: 'admin',
  capabilities: capabilities.toSet(),
);
