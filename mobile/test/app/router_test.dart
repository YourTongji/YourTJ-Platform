import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/app/router.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/forum/data/forum_repository.dart';
import 'package:yourtj_mobile/features/forum/domain/forum_route_filters.dart';

void main() {
  group('focusedSessionRedirect', () {
    test('keeps an account with required onboarding in the focused flow', () {
      final SessionState session = SessionState.authenticated(
        generation: 1,
        account: _account(onboardingRequired: true),
      );

      expect(
        focusedSessionRedirect(session, AppRoutes.forum),
        AppRoutes.onboarding,
      );
      expect(focusedSessionRedirect(session, AppRoutes.onboarding), isNull);
    });

    test('moves a completed account away from onboarding', () {
      final SessionState session = SessionState.authenticated(
        generation: 1,
        account: _account(onboardingRequired: false),
      );

      expect(
        focusedSessionRedirect(session, AppRoutes.onboarding),
        AppRoutes.account,
      );
      expect(focusedSessionRedirect(session, AppRoutes.wallet), isNull);
    });

    test('waits for restoration before deciding an onboarding deep link', () {
      expect(
        focusedSessionRedirect(
          const SessionState.restoring(generation: 0),
          AppRoutes.onboarding,
        ),
        isNull,
      );
    });

    test('sends an anonymous onboarding deep link to login', () {
      expect(
        focusedSessionRedirect(
          const SessionState.anonymous(generation: 1),
          AppRoutes.onboarding,
        ),
        AppRoutes.login,
      );
    });
  });

  group('authenticated return locations', () {
    test('keeps only bounded query fields for the message center', () {
      expect(
        safeAuthenticatedReturnLocation(
          '/messages?conversation=conversation-1&view=requests&token=secret',
        ),
        '/messages?conversation=conversation-1&view=requests',
      );
    });

    test('rejects external, public, fragmented, and oversized targets', () {
      expect(
        safeAuthenticatedReturnLocation('https://evil.example/wallet'),
        isNull,
      );
      expect(safeAuthenticatedReturnLocation('/forum'), isNull);
      expect(safeAuthenticatedReturnLocation('/wallet#token'), isNull);
      expect(
        safeAuthenticatedReturnLocation(
          '/${List<String>.filled(513, 'a').join()}',
        ),
        isNull,
      );
    });

    test(
      'redirects an anonymous protected deep link through focused login',
      () {
        expect(
          appSessionRedirect(
            const SessionState.anonymous(generation: 2),
            Uri.parse('/notifications?token=must-not-survive'),
          ),
          '/login?next=%2Fnotifications',
        );
        expect(
          appSessionRedirect(
            const SessionState.anonymous(generation: 2),
            Uri.parse('yourtj://app/wallet'),
          ),
          '/login?next=%2Fwallet',
        );
      },
    );

    test('rejects untrusted deep-link origins before route matching', () {
      const SessionState anonymous = SessionState.anonymous(generation: 2);

      expect(
        appSessionRedirect(anonymous, Uri.parse('yourtj://evil/wallet')),
        AppRoutes.rejectedLink,
      );
      expect(
        appSessionRedirect(anonymous, Uri.parse('https://evil.example/wallet')),
        AppRoutes.rejectedLink,
      );
      expect(
        appSessionRedirect(anonymous, Uri.parse('//evil.example/wallet')),
        AppRoutes.rejectedLink,
      );
      expect(
        appSessionRedirect(
          anonymous,
          Uri.parse('https://yourtj.de:444/wallet'),
        ),
        AppRoutes.rejectedLink,
      );
    });

    test('accepts the configured HTTPS universal-link origin', () {
      expect(
        appSessionRedirect(
          const SessionState.anonymous(generation: 2),
          Uri.parse('https://yourtj.de/notifications'),
        ),
        '/login?next=%2Fnotifications',
      );
    });

    test('carries the safe target through required onboarding', () {
      final SessionState session = SessionState.authenticated(
        generation: 3,
        account: _account(onboardingRequired: true),
      );

      expect(
        appSessionRedirect(
          session,
          Uri.parse('/login?next=%2Fsettings%2Fsessions'),
        ),
        '/onboarding?next=%2Fsettings%2Fsessions',
      );
    });
  });

  group('public interaction return locations', () {
    test('keeps only canonical public routes and bounded query fields', () {
      expect(
        safePublicInteractionReturnLocation(
          '/forum?tag=flutter-mobile&board=board-1&token=secret',
        ),
        '/forum?board=board-1&tag=flutter-mobile',
      );
      expect(
        safePublicInteractionReturnLocation('/forum/threads/thread_1'),
        '/forum/threads/thread_1',
      );
      expect(
        safePublicInteractionReturnLocation('/courses/course-1'),
        '/courses/course-1',
      );
      expect(
        safePublicInteractionReturnLocation(
          '/courses/course-1?review=review-7&token=secret',
        ),
        '/courses/course-1?review=review-7',
      );
      expect(
        safePublicInteractionReturnLocation('/profile/student_1/followers'),
        '/profile/student_1/followers',
      );
    });

    test('rejects external, auth, admin, sensitive, and oversized targets', () {
      for (final String location in <String>[
        'https://evil.example/forum',
        '//evil.example/forum',
        '/login',
        '/onboarding',
        '/admin',
        '/admin/reviews',
        '/wallet',
        '/messages?conversation=conversation-1',
        '/appeals',
        '/forum#access-token',
        '/${List<String>.filled(513, 'a').join()}',
      ]) {
        expect(
          safePublicInteractionReturnLocation(location),
          isNull,
          reason: location,
        );
      }
    });

    test('keeps public pages anonymous until a protected interaction', () {
      const SessionState anonymous = SessionState.anonymous(generation: 2);

      expect(appSessionRedirect(anonymous, Uri.parse('/forum')), isNull);
      expect(
        publicInteractionLoginLocation(
          Uri.parse('/forum?tag=flutter-mobile&board=board-1&token=secret'),
        ),
        '/login?next=%2Fforum%3Fboard%3Dboard-1%26tag%3Dflutter-mobile',
      );
      expect(
        publicInteractionLoginLocation(Uri.parse('https://evil.example/forum')),
        AppRoutes.login,
      );
    });

    test(
      'carries a public interaction target through login and onboarding',
      () {
        final String loginLocation = publicInteractionLoginLocation(
          Uri.parse('/courses/course-1'),
        );
        final SessionState onboardingRequired = SessionState.authenticated(
          generation: 3,
          account: _account(onboardingRequired: true),
        );

        expect(loginLocation, '/login?next=%2Fcourses%2Fcourse-1');
        expect(
          appSessionRedirect(onboardingRequired, Uri.parse(loginLocation)),
          '/onboarding?next=%2Fcourses%2Fcourse-1',
        );
        expect(
          safeLoginReturnLocation('/courses/course-1'),
          '/courses/course-1',
        );
      },
    );
  });

  testWidgets('unknown deep links fail closed without echoing the URI', (
    WidgetTester tester,
  ) async {
    final GoRouter router = createAppRouter(
      initialLocation: '/unsupported?token=must-not-render',
    );
    addTearDown(router.dispose);

    await tester.pumpWidget(
      YourTjApp(router: router, enableAnnouncementGate: false),
    );
    await tester.pumpAndSettle();

    expect(find.text('页面不存在'), findsOneWidget);
    expect(find.textContaining('must-not-render'), findsNothing);
    expect(find.byType(FilledButton), findsOneWidget);
  });

  test('forum query filters reject values outside their wire contracts', () {
    expect(ForumRouteFilters.boardId('board-1'), 'board-1');
    expect(ForumRouteFilters.boardId('../private'), isNull);
    expect(ForumRouteFilters.tagSlug('flutter-mobile'), 'flutter-mobile');
    expect(ForumRouteFilters.tagSlug('Flutter Mobile'), isNull);
  });

  testWidgets('forum route applies board and tag query filters to the feed', (
    WidgetTester tester,
  ) async {
    final _RecordingForumRepository repository = _RecordingForumRepository();
    final GoRouter router = createAppRouter(
      initialLocation: '/forum?board=board-2&tag=flutter-mobile',
    );
    addTearDown(router.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          forumRepositoryProvider.overrideWithValue(repository),
          sessionStateProvider.overrideWith(
            (Ref ref) => Stream<SessionState>.value(
              const SessionState.anonymous(generation: 1),
            ),
          ),
        ],
        child: YourTjApp(router: router, enableAnnouncementGate: false),
      ),
    );
    await tester.pumpAndSettle();

    expect(repository.requestedBoardId, 'board-2');
    expect(repository.requestedTag, 'flutter-mobile');
  });
}

Account _account({required bool onboardingRequired}) {
  return Account(
    id: 'account-1',
    handle: 'student',
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: const <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: onboardingRequired,
    createdAt: 1,
  );
}

class _RecordingForumRepository extends ForumRepository {
  _RecordingForumRepository() : super(ForumApi(Dio()));

  String? requestedBoardId;
  String? requestedTag;

  @override
  Future<List<Board>> boards() async => <Board>[
    Board(
      id: 'board-2',
      slug: 'board-2',
      name: '移动开发',
      parentId: null,
      description: null,
      position: 1,
      isLocked: false,
      minTrustToPost: 1,
      isQa: false,
      threadCount: 0,
      canPost: false,
      postingRestriction: BoardPostingRestrictionEnum.loginRequired,
    ),
  ];

  @override
  Future<List<Tag>> tags() async => <Tag>[
    Tag(id: 'tag-1', slug: 'flutter-mobile', name: 'Flutter Mobile'),
  ];

  @override
  Future<ForumPageSlice<ThreadFeed>> threads({
    required ForumFeed feed,
    String? boardId,
    String? tag,
    String? cursor,
  }) async {
    requestedBoardId = boardId;
    requestedTag = tag;
    return const ForumPageSlice<ThreadFeed>(
      items: <ThreadFeed>[],
      nextCursor: null,
      hasMore: false,
    );
  }
}
