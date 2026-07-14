import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/widgets/platform_avatar.dart';
import 'package:yourtj_mobile/features/forum/presentation/forum_widgets.dart';

void main() {
  testWidgets('thread card exposes feed metadata and opens detail', (
    WidgetTester tester,
  ) async {
    final GoRouter router = GoRouter(
      initialLocation: '/forum',
      routes: <RouteBase>[
        GoRoute(
          path: '/forum',
          builder: (BuildContext context, GoRouterState state) => Scaffold(
            body: ForumThreadCard(thread: _thread(), boardName: '校园生活'),
          ),
          routes: <RouteBase>[
            GoRoute(
              path: 'threads/:threadId',
              builder: (BuildContext context, GoRouterState state) => Scaffold(
                body: Text('主题 ${state.pathParameters['threadId']}'),
              ),
            ),
          ],
        ),
      ],
    );
    addTearDown(router.dispose);

    await tester.pumpWidget(MaterialApp.router(routerConfig: router));

    expect(find.text('移动端对齐'), findsOneWidget);
    expect(find.text('校园生活'), findsOneWidget);
    expect(find.text('#Flutter'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(find.text('7'), findsOneWidget);
    expect(find.byType(PlatformAvatar), findsOneWidget);

    await tester.tap(find.text('移动端对齐'));
    await tester.pumpAndSettle();

    expect(find.text('主题 thread-1'), findsOneWidget);
  });

  testWidgets(
    'forum body filters stale and unsafe deliveries then refreshes once',
    (WidgetTester tester) async {
      int refreshes = 0;
      final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ForumBody(
              source: '![stale](yourtj-asset:1) ![unsafe](yourtj-asset:2)',
              format: ContentFormat.markdownV1,
              attachments: <ForumAttachment>[
                _attachment(
                  assetId: '1',
                  url: 'https://media.yourtj.de/stale.webp',
                  expiresAt: now - 1,
                ),
                _attachment(
                  assetId: '2',
                  url: 'http://media.yourtj.de/unsafe.webp',
                  expiresAt: now + 300,
                ),
              ],
              onRefreshDelivery: () => refreshes += 1,
            ),
          ),
        ),
      );
      await tester.pump();
      await tester.pump();

      expect(find.textContaining('图片当前不可用'), findsNWidgets(2));
      expect(find.byType(Image), findsNothing);
      expect(refreshes, 1);
    },
  );

  testWidgets('stale attachment image refreshes its owning resource once', (
    WidgetTester tester,
  ) async {
    int refreshes = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ForumAttachmentImage(
            attachment: _attachment(
              assetId: '1',
              url: 'https://media.yourtj.de/stale.webp',
              expiresAt: DateTime.now().millisecondsSinceEpoch ~/ 1000 - 1,
            ),
            onRefreshDelivery: () => refreshes += 1,
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(find.text('图片链接已失效，刷新后查看'), findsOneWidget);
    expect(refreshes, 1);
  });
}

ThreadFeed _thread() {
  final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
  return ThreadFeed(
    id: 'thread-1',
    boardId: 'board-1',
    authorHandle: 'alice',
    authorAvatar: null,
    title: '移动端对齐',
    bodyExcerpt: '同一套产品能力与视觉语言。',
    contentVersion: 1,
    replyCount: 7,
    voteCount: 3,
    hotScore: 8,
    status: ThreadFeedStatusEnum.visible,
    createdAt: now - 60,
    lastActivityAt: now,
    tags: <String>['Flutter'],
    attachments: <ForumAttachment>[],
    viewerVote: null,
    isBookmarked: false,
    canEdit: false,
    canDelete: false,
    canModerate: false,
  );
}

ForumAttachment _attachment({
  required String assetId,
  required String url,
  required int expiresAt,
}) {
  return ForumAttachment(
    assetId: assetId,
    reference: 'yourtj-asset:$assetId',
    position: int.parse(assetId) - 1,
    alt: '测试图片',
    url: url,
    expiresAt: expiresAt,
    width: 320,
    height: 240,
  );
}
