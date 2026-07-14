import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/account/data/account_repository.dart';

import '../../auth/support/session_test_support.dart';

void main() {
  test(
    'onboarding sends the generated contract shape and returns server facts',
    () async {
      late final RecordingAdapter adapter;
      final AccountRepository repository = _repository((
        RequestOptions options,
      ) {
        expect(options.path, '/me/onboarding');
        expect(options.method, 'PUT');
        return jsonResponse(<String, Object?>{
          'required': false,
          'currentTermsVersion': 'terms-2026-07',
          'acceptedTermsVersion': 'terms-2026-07',
          'handle': 'student.one',
          'displayName': 'Student One',
          'bio': null,
          'profileVisibility': 'campus',
          'activityVisibility': 'only_me',
          'discoverable': false,
          'completedAt': 100,
        });
      }, onAdapter: (RecordingAdapter value) => adapter = value);

      final OnboardingState result = await repository.completeOnboarding(
        OnboardingCompleteInput(
          handle: 'student.one',
          displayName: 'Student One',
          bio: null,
          profileVisibility: ProfileVisibility.campus,
          activityVisibility: ActivityVisibility.onlyMe,
          discoverable: false,
          acceptedTermsVersion: 'terms-2026-07',
        ),
      );

      expect(result.required_, isFalse);
      expect(result.completedAt, 100);
      expect(requestJson(adapter.requests.single), <String, Object?>{
        'handle': 'student.one',
        'displayName': 'Student One',
        'bio': null,
        'profileVisibility': 'campus',
        'activityVisibility': 'only_me',
        'discoverable': false,
        'acceptedTermsVersion': 'terms-2026-07',
      });
    },
  );

  test(
    'deletion keeps one idempotency key and explicit DELETE confirmation',
    () async {
      late final RecordingAdapter adapter;
      const String recoveryToken =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ';
      final AccountRepository repository = _repository((
        RequestOptions options,
      ) {
        expect(options.path, '/me/lifecycle/delete');
        return jsonResponse(<String, Object?>{
          'lifecycle': _lifecycleJson(
            state: 'deletion_requested',
            recoverUntil: 300,
          ),
          'recovery': <String, Object?>{
            'recoveryToken': recoveryToken,
            'expiresAt': 200,
            'lifecycle': _lifecycleJson(
              state: 'deletion_requested',
              recoverUntil: 300,
            ),
          },
        }, statusCode: 202);
      }, onAdapter: (RecordingAdapter value) => adapter = value);

      final AccountLifecycleMutation result = await repository
          .requestAccountDeletion('7ff7728a-22b7-4d0f-97ac-c292b18a7720');

      final RecordedRequest request = adapter.requests.single;
      expect(
        request.headers['Idempotency-Key'],
        '7ff7728a-22b7-4d0f-97ac-c292b18a7720',
      );
      expect(requestJson(request), <String, Object?>{'confirmation': 'DELETE'});
      expect(result.lifecycle.state, AccountLifecycleState.deletionRequested);
      expect(result.recovery.recoveryToken, recoveryToken);
    },
  );

  test(
    'recent-auth failure preserves the bounded server recovery message',
    () async {
      final AccountRepository repository = _repository(
        (RequestOptions options) => jsonResponse(<String, Object?>{
          'error': <String, String>{
            'code': 'RECENT_AUTH_REQUIRED',
            'message': '请先完成最近认证',
          },
        }, statusCode: 428),
      );

      await expectLater(
        repository.createDataExport('same-key-on-retry'),
        throwsA(
          isA<ApiFailure>()
              .having((ApiFailure failure) => failure.statusCode, 'status', 428)
              .having(
                (ApiFailure failure) => failure.code,
                'code',
                'RECENT_AUTH_REQUIRED',
              )
              .having(
                (ApiFailure failure) => failure.message,
                'message',
                '请先完成最近认证',
              ),
        ),
      );
    },
  );

  test(
    'recovery uses a purpose-bound header without a bearer credential',
    () async {
      late final RecordingAdapter adapter;
      const String recoveryToken =
          'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ';
      final AccountRepository repository = _repository((
        RequestOptions options,
      ) {
        expect(options.path, '/auth/recovery');
        expect(options.method, 'POST');
        return jsonResponse(_lifecycleJson(state: 'active'));
      }, onAdapter: (RecordingAdapter value) => adapter = value);

      final AccountLifecycle result = await repository.recoverAccount(
        recoveryToken,
      );

      final RecordedRequest request = adapter.requests.single;
      expect(request.headers['X-Recovery-Token'], recoveryToken);
      expect(request.headers, isNot(contains('Authorization')));
      expect(request.extra['secure'], isEmpty);
      expect(result.state, AccountLifecycleState.active);
    },
  );

  test(
    'notification preference replacement serializes every stored choice',
    () async {
      late final RecordingAdapter adapter;
      final AccountRepository repository = _repository((
        RequestOptions options,
      ) {
        expect(options.path, '/me/notification-prefs');
        return jsonResponse(<String, Object?>{
          'prefs': <String, Object?>{
            'inApp': <String, bool>{
              'replies': false,
              'mentions': true,
              'quotes': false,
              'votes': true,
              'badges': false,
              'follows': true,
              'subscriptions': false,
              'directMessages': true,
            },
            'email': <String, bool>{'weeklyDigest': true},
          },
        });
      }, onAdapter: (RecordingAdapter value) => adapter = value);

      final NotificationPrefs response = await repository
          .updateNotificationPreferences(
            NotificationPrefsInput(
              prefs: NotificationPreferencesInput(
                inApp: InAppNotificationPrefsInput(
                  replies: false,
                  mentions: true,
                  quotes: false,
                  votes: true,
                  badges: false,
                  follows: true,
                  subscriptions: false,
                  directMessages: true,
                ),
                email: EmailNotificationPrefs(weeklyDigest: true),
              ),
            ),
          );

      expect(response.prefs.email.weeklyDigest, isTrue);
      expect(requestJson(adapter.requests.single), <String, Object?>{
        'prefs': <String, Object?>{
          'inApp': <String, bool>{
            'replies': false,
            'mentions': true,
            'quotes': false,
            'votes': true,
            'badges': false,
            'follows': true,
            'subscriptions': false,
            'directMessages': true,
          },
          'email': <String, bool>{'weeklyDigest': true},
        },
      });
    },
  );
}

AccountRepository _repository(
  AdapterHandler handler, {
  void Function(RecordingAdapter adapter)? onAdapter,
}) {
  final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
  final RecordingAdapter adapter = RecordingAdapter(handler);
  dio.httpClientAdapter = adapter;
  onAdapter?.call(adapter);
  return AccountRepository(
    IdentityApi(dio),
    AuthApi(dio),
    NotificationsApi(dio),
    MediaApi(dio),
  );
}

Map<String, Object?> _lifecycleJson({
  required String state,
  int? recoverUntil,
}) {
  return <String, Object?>{
    'state': state,
    'deactivatedAt': state == 'deactivated' ? 100 : null,
    'deletionRequestedAt': state == 'deletion_requested' ? 100 : null,
    'recoverUntil': recoverUntil,
    'deletedAt': state == 'deleted' ? 120 : null,
    'purgedAt': state == 'purged' ? 140 : null,
    'lifecycleVersion': 3,
  };
}
