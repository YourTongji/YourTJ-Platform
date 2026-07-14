import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/forum/data/forum_repository.dart';
import 'package:yourtj_mobile/features/forum/presentation/thread_detail_page.dart';

void main() {
  testWidgets('thread and session switch discard a late draft response', (
    WidgetTester tester,
  ) async {
    final Completer<DraftOutput?> first = Completer<DraftOutput?>();
    final Completer<DraftOutput?> second = Completer<DraftOutput?>();
    final _DraftRepository repository = _DraftRepository(<Future<DraftOutput?>>[
      first.future,
      second.future,
    ]);
    late StateSetter updateHost;
    String threadId = 'thread-1';
    int sessionGeneration = 1;

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: AppTheme.light,
          home: Scaffold(
            body: SingleChildScrollView(
              child: StatefulBuilder(
                builder: (BuildContext context, StateSetter setState) {
                  updateHost = setState;
                  return CommentComposer(
                    threadId: threadId,
                    authenticated: true,
                    sessionGeneration: sessionGeneration,
                    repository: repository,
                    onLogin: () {},
                    onPosted: () async {},
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(repository.requestedKeys, <String>['comment:thread-1']);

    updateHost(() {
      threadId = 'thread-2';
      sessionGeneration = 2;
    });
    await tester.pump();
    expect(repository.requestedKeys, <String>[
      'comment:thread-1',
      'comment:thread-2',
    ]);

    second.complete(_draft('thread-2', '新账号的新主题草稿'));
    await tester.pump();
    await tester.pump();
    expect(_composerText(tester), '新账号的新主题草稿');

    first.complete(_draft('thread-1', '旧账号迟到草稿'));
    await tester.pump();
    await tester.pump();

    expect(_composerText(tester), '新账号的新主题草稿');
    expect(find.textContaining('旧账号迟到草稿'), findsNothing);
  });
}

String _composerText(WidgetTester tester) {
  final TextField field = tester.widget<TextField>(find.byType(TextField));
  return field.controller!.text;
}

DraftOutput _draft(String threadId, String body) {
  return DraftOutput(
    draftKey: 'comment:$threadId',
    payload: ForumDraftPayload.comment(
      CommentDraftPayload(
        kind: CommentDraftPayloadKindEnum.comment,
        threadId: threadId,
        body: body,
        contentFormat: ContentFormat.markdownV1,
        parentId: null,
        attachmentAssetIds: <String>{},
      ),
    ),
    version: 1,
    updatedAt: 100,
  );
}

class _DraftRepository extends ForumRepository {
  _DraftRepository(this.responses) : super(ForumApi(Dio()));

  final List<Future<DraftOutput?>> responses;
  final List<String> requestedKeys = <String>[];

  @override
  Future<DraftOutput?> draft(String key) {
    requestedKeys.add(key);
    return responses[requestedKeys.length - 1];
  }
}
