import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/forum/data/forum_repository.dart';
import 'package:yourtj_mobile/features/forum/presentation/revision_history_sheet.dart';

void main() {
  testWidgets('history paginates with stable cursor and renders old source', (
    WidgetTester tester,
  ) async {
    final _RevisionRepository repository = _RevisionRepository((
      String? cursor,
    ) async {
      if (cursor == null) {
        return ForumPageSlice<PostRevision>(
          items: <PostRevision>[_revision('revision-1', body: '**第一页**')],
          nextCursor: 'cursor-2',
          hasMore: true,
        );
      }
      expect(cursor, 'cursor-2');
      return ForumPageSlice<PostRevision>(
        items: <PostRevision>[_revision('revision-2', body: '第二页')],
        nextCursor: null,
        hasMore: false,
      );
    });

    await _pumpHistory(tester, repository: repository);
    await tester.pumpAndSettle();

    expect(find.text('第一页'), findsOneWidget);
    expect(find.text('加载更多历史'), findsOneWidget);
    await tester.tap(find.text('加载更多历史'));
    await tester.pumpAndSettle();

    expect(repository.cursors, <String?>[null, 'cursor-2']);
    expect(find.text('编辑前版本 v1'), findsNWidgets(2));
    await tester.tap(find.text('编辑前版本 v1').last);
    await tester.pumpAndSettle();
    expect(find.text('第二页'), findsOneWidget);
    expect(find.text('加载更多历史'), findsNothing);
  });

  testWidgets('empty and forbidden histories have distinct truthful states', (
    WidgetTester tester,
  ) async {
    final _RevisionRepository emptyRepository = _RevisionRepository(
      (String? cursor) async => const ForumPageSlice<PostRevision>(
        items: <PostRevision>[],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await _pumpHistory(tester, repository: emptyRepository);
    await tester.pumpAndSettle();
    expect(find.text('暂无修订历史'), findsOneWidget);

    final _RevisionRepository forbiddenRepository = _RevisionRepository(
      (String? cursor) async => throw const ApiFailure(
        kind: ApiFailureKind.forbidden,
        message: '仅作者或满足层级约束的工作人员可读',
      ),
    );
    await _pumpHistory(tester, repository: forbiddenRepository);
    await tester.pumpAndSettle();
    expect(find.text('无权查看修订历史'), findsOneWidget);
    expect(find.text('仅作者或满足层级约束的工作人员可读'), findsOneWidget);
  });

  testWidgets('error state retries without replacing it with fake history', (
    WidgetTester tester,
  ) async {
    int calls = 0;
    final _RevisionRepository repository = _RevisionRepository((
      String? cursor,
    ) async {
      calls += 1;
      if (calls == 1) {
        throw const ApiFailure(kind: ApiFailureKind.server, message: '服务暂时不可用');
      }
      return ForumPageSlice<PostRevision>(
        items: <PostRevision>[_revision('revision-after-retry')],
        nextCursor: null,
        hasMore: false,
      );
    });

    await _pumpHistory(tester, repository: repository);
    await tester.pumpAndSettle();
    expect(find.text('修订历史加载失败'), findsOneWidget);

    await tester.tap(find.text('重试'));
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.text('编辑前版本 v1'), findsOneWidget);
  });

  testWidgets('expired revision delivery refreshes the owning page once', (
    WidgetTester tester,
  ) async {
    int calls = 0;
    final _RevisionRepository repository = _RevisionRepository((
      String? cursor,
    ) async {
      calls += 1;
      return ForumPageSlice<PostRevision>(
        items: <PostRevision>[
          _revision(
            'revision-with-image',
            body: '![历史图片](yourtj-asset:1)',
            attachments: <ForumAttachment>[
              ForumAttachment(
                assetId: '1',
                reference: 'yourtj-asset:1',
                position: 0,
                alt: '历史图片',
                url: 'https://media.yourtj.de/expired.webp',
                expiresAt: DateTime.now().millisecondsSinceEpoch ~/ 1000 - 1,
                width: 320,
                height: 240,
              ),
            ],
          ),
        ],
        nextCursor: null,
        hasMore: false,
      );
    });

    await _pumpHistory(tester, repository: repository);
    await tester.pumpAndSettle();
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.textContaining('图片当前不可用'), findsOneWidget);
  });

  testWidgets('session switch discards a late response from the old account', (
    WidgetTester tester,
  ) async {
    final StreamController<SessionState> sessions =
        StreamController<SessionState>();
    final Completer<ForumPageSlice<PostRevision>> first =
        Completer<ForumPageSlice<PostRevision>>();
    final Completer<ForumPageSlice<PostRevision>> second =
        Completer<ForumPageSlice<PostRevision>>();
    int calls = 0;
    final _RevisionRepository repository = _RevisionRepository((
      String? cursor,
    ) {
      calls += 1;
      return calls == 1 ? first.future : second.future;
    });
    addTearDown(sessions.close);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          sessionStateProvider.overrideWith((Ref ref) => sessions.stream),
        ],
        child: MaterialApp(
          theme: AppTheme.light,
          home: ForumRevisionHistorySheet(
            repository: repository,
            target: ForumRevisionTarget.thread,
            targetId: 'thread-1',
          ),
        ),
      ),
    );
    sessions.add(
      SessionState.authenticated(generation: 1, account: _account('account-1')),
    );
    await tester.pump();
    await tester.pump();
    expect(find.text('加载修订历史'), findsOneWidget);
    expect(calls, 1);

    sessions.add(
      SessionState.authenticated(generation: 2, account: _account('account-2')),
    );
    await tester.pump();
    await tester.pump();
    expect(calls, 2);

    second.complete(
      ForumPageSlice<PostRevision>(
        items: <PostRevision>[_revision('new-account', body: '新账号可见历史')],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('新账号可见历史'), findsOneWidget);

    first.complete(
      ForumPageSlice<PostRevision>(
        items: <PostRevision>[_revision('old-account', body: '旧账号迟到历史')],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('新账号可见历史'), findsOneWidget);
    expect(find.text('旧账号迟到历史'), findsNothing);
  });
}

Future<void> _pumpHistory(
  WidgetTester tester, {
  required ForumRepository repository,
}) {
  return tester.pumpWidget(
    ProviderScope(
      overrides: [
        sessionStateProvider.overrideWith(
          (Ref ref) => Stream<SessionState>.value(
            SessionState.authenticated(
              generation: 1,
              account: _account('account-1'),
            ),
          ),
        ),
      ],
      child: MaterialApp(
        theme: AppTheme.light,
        home: ForumRevisionHistorySheet(
          repository: repository,
          target: ForumRevisionTarget.thread,
          targetId: 'thread-1',
        ),
      ),
    ),
  );
}

class _RevisionRepository extends ForumRepository {
  _RevisionRepository(this.loader) : super(ForumApi(Dio()));

  final Future<ForumPageSlice<PostRevision>> Function(String? cursor) loader;
  final List<String?> cursors = <String?>[];

  @override
  Future<ForumPageSlice<PostRevision>> threadRevisions(
    String threadId, {
    String? cursor,
  }) {
    cursors.add(cursor);
    return loader(cursor);
  }
}

PostRevision _revision(
  String id, {
  String body = '历史正文',
  List<ForumAttachment> attachments = const <ForumAttachment>[],
}) {
  return PostRevision(
    id: id,
    seq: 1,
    editorId: 'account-1',
    oldTitle: '历史标题',
    oldBody: body,
    oldContentFormat: ContentFormat.markdownV1,
    oldContentVersion: 1,
    attachments: attachments,
    createdAt: 100,
  );
}

Account _account(String id) {
  return Account(
    id: id,
    handle: id,
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: const <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 1,
  );
}
