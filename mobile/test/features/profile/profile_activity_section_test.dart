import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/forum/presentation/forum_widgets.dart';
import 'package:yourtj_mobile/features/profile/data/profile_activity_repository.dart';
import 'package:yourtj_mobile/features/profile/domain/profile_activity_controller.dart';
import 'package:yourtj_mobile/features/profile/presentation/profile_activity_section.dart';

void main() {
  testWidgets('renders loading, real tabs, typed media and detail navigation', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(320, 800);
    addTearDown(tester.view.reset);
    final Completer<ProfileActivityPage<UserThread>> threadRequest =
        Completer<ProfileActivityPage<UserThread>>();
    final _SectionSource source = _SectionSource();
    source.onThreads = (String handle, String? cursor) => threadRequest.future;
    source.onComments = (String handle, String? cursor) async =>
        const ProfileActivityPage<UserComment>(
          items: <UserComment>[],
          nextCursor: null,
          hasMore: false,
        );
    source.onMedia = (String handle, String? cursor) async =>
        ProfileActivityPage<ProfileContent>(
          items: <ProfileContent>[_content()],
          nextCursor: null,
          hasMore: false,
        );
    source.onLikes = (String handle, String? cursor) async =>
        throw const ApiFailure(
          kind: ApiFailureKind.forbidden,
          message: '该用户没有公开喜欢列表',
        );
    final ProfileActivityController controller = ProfileActivityController(
      source,
    );
    addTearDown(controller.dispose);
    controller.configure(
      handle: 'alice',
      viewerKey: 'anonymous',
      canViewActivity: true,
    );
    final Future<void> loading = controller.loadSelected();
    final GoRouter router = _router(controller);
    addTearDown(router.dispose);

    await tester.pumpWidget(
      MaterialApp.router(theme: AppTheme.light, routerConfig: router),
    );
    expect(find.text('正在加载公开主题'), findsOneWidget);

    threadRequest.complete(
      ProfileActivityPage<UserThread>(
        items: <UserThread>[_thread()],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await loading;
    await tester.pump();

    expect(find.text('真实主题'), findsOneWidget);
    expect(find.byKey(const Key('profile-activity-comments')), findsOneWidget);
    expect(find.byKey(const Key('profile-activity-media')), findsOneWidget);
    expect(find.byKey(const Key('profile-activity-likes')), findsOneWidget);

    await tester.tap(find.text('真实主题'));
    await tester.pumpAndSettle();
    expect(find.text('主题详情 thread-1'), findsOneWidget);

    router.go('/profile');
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('profile-activity-comments')));
    await tester.pumpAndSettle();
    expect(find.text('暂无公开回复'), findsOneWidget);

    await tester.tap(find.byKey(const Key('profile-activity-media')));
    await tester.pump();
    await tester.pump();
    expect(find.text('校园相册'), findsOneWidget);
    expect(find.byType(ForumBody), findsOneWidget);
    expect(find.byType(ForumAttachmentImage), findsOneWidget);

    await tester.tap(find.byKey(const Key('profile-activity-likes')));
    await tester.pumpAndSettle();
    expect(find.text('活动列表不可见'), findsOneWidget);
    expect(find.text('该用户没有公开喜欢列表'), findsOneWidget);
  });

  testWidgets('private activity shows permission state without a request', (
    WidgetTester tester,
  ) async {
    final _SectionSource source = _SectionSource();
    final ProfileActivityController controller = ProfileActivityController(
      source,
    );
    addTearDown(controller.dispose);
    controller.configure(
      handle: 'alice',
      viewerKey: 'anonymous',
      canViewActivity: false,
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.light,
        home: Scaffold(body: ProfileActivitySection(controller: controller)),
      ),
    );

    expect(find.text('活动列表未公开'), findsOneWidget);
    expect(source.calls, 0);
  });
}

GoRouter _router(ProfileActivityController controller) {
  return GoRouter(
    initialLocation: '/profile',
    routes: <RouteBase>[
      GoRoute(
        path: '/profile',
        builder: (BuildContext context, GoRouterState state) => Scaffold(
          body: SingleChildScrollView(
            padding: const EdgeInsets.all(16),
            child: ProfileActivitySection(controller: controller),
          ),
        ),
      ),
      GoRoute(
        path: '/forum/threads/:threadId',
        builder: (BuildContext context, GoRouterState state) =>
            Scaffold(body: Text('主题详情 ${state.pathParameters['threadId']}')),
      ),
    ],
  );
}

class _SectionSource implements ProfileActivitySource {
  Future<ProfileActivityPage<UserThread>> Function(String, String?)? onThreads;
  Future<ProfileActivityPage<UserComment>> Function(String, String?)?
  onComments;
  Future<ProfileActivityPage<ProfileContent>> Function(String, String?)?
  onMedia;
  Future<ProfileActivityPage<ProfileContent>> Function(String, String?)?
  onLikes;
  int calls = 0;

  @override
  Future<ProfileActivityPage<UserThread>> threads({
    required String handle,
    String? cursor,
  }) {
    calls += 1;
    return onThreads?.call(handle, cursor) ??
        Future<ProfileActivityPage<UserThread>>.value(
          const ProfileActivityPage<UserThread>(
            items: <UserThread>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }

  @override
  Future<ProfileActivityPage<UserComment>> comments({
    required String handle,
    String? cursor,
  }) {
    calls += 1;
    return onComments?.call(handle, cursor) ??
        Future<ProfileActivityPage<UserComment>>.value(
          const ProfileActivityPage<UserComment>(
            items: <UserComment>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }

  @override
  Future<ProfileActivityPage<ProfileContent>> media({
    required String handle,
    String? cursor,
  }) {
    calls += 1;
    return onMedia?.call(handle, cursor) ??
        Future<ProfileActivityPage<ProfileContent>>.value(
          const ProfileActivityPage<ProfileContent>(
            items: <ProfileContent>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }

  @override
  Future<ProfileActivityPage<ProfileContent>> likes({
    required String handle,
    String? cursor,
  }) {
    calls += 1;
    return onLikes?.call(handle, cursor) ??
        Future<ProfileActivityPage<ProfileContent>>.value(
          const ProfileActivityPage<ProfileContent>(
            items: <ProfileContent>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }
}

UserThread _thread() => UserThread(
  id: 'thread-1',
  title: '真实主题',
  bodyExcerpt: '主题摘要',
  contentFormat: ContentFormat.plainV1,
  boardSlug: 'campus',
  replyCount: 2,
  voteCount: 3,
  viewerVote: null,
  isBookmarked: false,
  attachments: const <ForumAttachment>[],
  createdAt: 100,
);

ProfileContent _content() => ProfileContent(
  targetType: ProfileContentTargetTypeEnum.thread,
  id: 'media-1',
  threadId: 'thread-media',
  title: '校园相册',
  body: '![校园](yourtj-asset:1)',
  contentFormat: ContentFormat.markdownV1,
  boardSlug: 'campus',
  authorHandle: 'alice',
  authorDisplayName: 'Alice',
  replyCount: 1,
  voteCount: 6,
  viewerVote: null,
  isBookmarked: true,
  attachments: <ForumAttachment>[
    ForumAttachment(
      assetId: '1',
      reference: 'yourtj-asset:1',
      position: 0,
      alt: '校园',
      url: 'https://media.yourtj.de/campus.webp',
      expiresAt: DateTime.now().millisecondsSinceEpoch ~/ 1000 + 300,
      width: 640,
      height: 480,
    ),
  ],
  createdAt: 100,
  activityAt: 110,
);
