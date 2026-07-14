import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/admin/data/admin_repository.dart';
import 'package:yourtj_mobile/features/admin/domain/admin_capabilities.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test('repository rejects a section before touching the network', () async {
    final _FakeAdminDataSource source = _FakeAdminDataSource();
    final AdminRepository repository = AdminRepository(source);

    await expectLater(
      repository.load(
        AdminSection.audit,
        _actor(const <String>[AdminCapabilities.searchUsers]),
      ),
      throwsA(isA<AdminAccessDenied>()),
    );
    expect(source.calls, isEmpty);
  });

  test(
    'repository passes only an authorized section to the data source',
    () async {
      final _FakeAdminDataSource source = _FakeAdminDataSource();
      final AdminRepository repository = AdminRepository(source);

      final AdminSectionSnapshot result = await repository.load(
        AdminSection.audit,
        _actor(const <String>[AdminCapabilities.readAudit]),
      );

      expect(result.section, AdminSection.audit);
      expect(source.calls, <AdminSection>[AdminSection.audit]);
    },
  );

  test('recent-auth response becomes an explicit recoverable gate', () async {
    final _FakeAdminDataSource source = _FakeAdminDataSource(
      error: _dioError(428, 'RECENT_AUTH_REQUIRED'),
    );
    final AdminRepository repository = AdminRepository(source);

    await expectLater(
      repository.load(
        AdminSection.system,
        _actor(const <String>[AdminCapabilities.runOperations]),
      ),
      throwsA(isA<AdminRecentAuthenticationRequired>()),
    );
  });

  test(
    'conflict response remains conflict and is never reported as success',
    () async {
      final _FakeAdminDataSource source = _FakeAdminDataSource(
        error: _dioError(409, 'VERSION_CONFLICT'),
      );
      final AdminRepository repository = AdminRepository(source);

      await expectLater(
        repository.load(
          AdminSection.appeals,
          _actor(const <String>[AdminCapabilities.reviewAppeals]),
        ),
        throwsA(
          isA<ApiFailure>()
              .having(
                (ApiFailure failure) => failure.kind,
                'kind',
                ApiFailureKind.conflict,
              )
              .having(
                (ApiFailure failure) => failure.code,
                'code',
                'VERSION_CONFLICT',
              ),
        ),
      );
    },
  );

  test(
    'generated AdminApi overview is decoded into evidence records',
    () async {
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      late final RecordingAdapter adapter;
      adapter = RecordingAdapter((RequestOptions options) {
        expect(options.path, '/admin/overview');
        expect(options.method, 'GET');
        return jsonResponse(<String, Object>{
          'totalUsers': 100,
          'activeUsers': 40,
          'suspendedUsers': 2,
          'moderators': 3,
          'administrators': 1,
          'pendingReviewReports': 4,
          'pendingForumFlags': 5,
          'pendingDmReports': 6,
          'pendingMediaUploads': 7,
          'threadsToday': 8,
          'commentsToday': 9,
          'likesToday': 10,
        });
      });
      dio.httpClientAdapter = adapter;
      final AdminRepository repository = AdminRepository(
        GeneratedAdminReadDataSource(AdminApi(dio)),
      );

      final AdminSectionSnapshot snapshot = await repository.load(
        AdminSection.overview,
        _actor(const <String>[AdminCapabilities.searchUsers]),
      );

      expect(adapter.requests, hasLength(1));
      expect(snapshot.groups, hasLength(2));
      expect(snapshot.groups.first.records.first.title, '100');
      expect(snapshot.groups.last.records.last.title, '7');
      dio.close(force: true);
    },
  );

  test('non-search user capability does not over-read the directory', () async {
    final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
    final RecordingAdapter adapter = RecordingAdapter(
      (RequestOptions options) =>
          jsonResponse(<String, Object>{}, statusCode: 500),
    );
    dio.httpClientAdapter = adapter;
    final AdminRepository repository = AdminRepository(
      GeneratedAdminReadDataSource(AdminApi(dio)),
    );

    final AdminSectionSnapshot snapshot = await repository.load(
      AdminSection.users,
      _actor(const <String>[AdminCapabilities.inviteUsers]),
    );

    expect(adapter.requests, isEmpty);
    expect(snapshot.groups.single.records, isEmpty);
    expect(snapshot.groups.single.description, contains('users.search'));
    dio.close(force: true);
  });
}

class _FakeAdminDataSource implements AdminReadDataSource {
  _FakeAdminDataSource({this.error});

  final Object? error;
  final List<AdminSection> calls = <AdminSection>[];

  @override
  Future<AdminSectionSnapshot> load(
    AdminSection section,
    AdminActorContext actor,
  ) async {
    calls.add(section);
    final Object? failure = error;
    if (failure != null) {
      throw failure;
    }
    return AdminSectionSnapshot(
      section: section,
      groups: const <AdminRecordGroup>[],
      loadedAt: DateTime.fromMillisecondsSinceEpoch(0),
    );
  }
}

AdminActorContext _actor(Iterable<String> capabilities) => AdminActorContext(
  accountId: 'actor-id',
  role: 'admin',
  capabilities: capabilities.toSet(),
);

DioException _dioError(int statusCode, String code) {
  final RequestOptions request = RequestOptions(path: '/admin/test');
  return DioException.badResponse(
    statusCode: statusCode,
    requestOptions: request,
    response: Response<Object>(
      requestOptions: request,
      statusCode: statusCode,
      data: <String, Object>{
        'error': <String, String>{'code': code, 'message': 'server message'},
      },
    ),
  );
}
