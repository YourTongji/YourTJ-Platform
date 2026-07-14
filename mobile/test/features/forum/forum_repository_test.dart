import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/forum/data/forum_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test('authenticated forum feeds are identified before issuing a request', () {
    expect(ForumFeed.hot.requiresAuthentication, isFalse);
    expect(ForumFeed.newest.requiresAuthentication, isFalse);
    expect(ForumFeed.subscriptions.requiresAuthentication, isTrue);
    expect(ForumFeed.following.requiresAuthentication, isTrue);
    expect(ForumFeed.unread.requiresAuthentication, isTrue);
    expect(ForumFeed.newest.wireName, 'new');
  });

  test('thread draft union stays wire-compatible with the web client', () {
    final DraftSaveInput input = DraftSaveInput(
      draftKey: 'thread:new',
      expectedVersion: 4,
      payload: ForumDraftPayload.thread(
        ThreadDraftPayload(
          kind: ThreadDraftPayloadKindEnum.thread,
          boardId: 'campus',
          title: '标题',
          body: '正文',
          contentFormat: ContentFormat.markdownV1,
          tags: <String>['同济'],
          pollQuestion: '',
          pollOptions: <String>[],
          attachmentAssetIds: <String>{},
        ),
      ),
    );

    final Map<String, dynamic> json = input.toJson();
    final Map<String, dynamic> payload =
        json['payload']! as Map<String, dynamic>;

    expect(json['draftKey'], 'thread:new');
    expect(json['expectedVersion'], 4);
    expect(payload['kind'], 'thread');
    expect(payload['boardId'], 'campus');
    expect(payload['contentFormat'], 'markdown_v1');
    expect(payload.containsKey('payload'), isFalse);
  });

  test('comment draft union keeps nullable parent and local text', () {
    final ForumDraftPayload draft = ForumDraftPayload.comment(
      CommentDraftPayload(
        kind: CommentDraftPayloadKindEnum.comment,
        threadId: 'thread-1',
        body: '未发布输入',
        contentFormat: ContentFormat.markdownV1,
        parentId: null,
        attachmentAssetIds: <String>{},
      ),
    );

    expect(draft.toJson(), <String, dynamic>{
      'kind': 'comment',
      'threadId': 'thread-1',
      'body': '未发布输入',
      'contentFormat': 'markdown_v1',
      'parentId': null,
      'attachmentAssetIds': <String>[],
    });
  });

  test('revision history uses typed bounded cursor routes', () async {
    final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
    final RecordingAdapter adapter = RecordingAdapter((RequestOptions options) {
      if (options.path == '/forum/threads/thread-1/revisions') {
        return jsonResponse(<String, Object?>{
          'items': <Object?>[_revisionJson(id: 'revision-1')],
          'nextCursor': 'next-page',
          'hasMore': true,
        });
      }
      expect(options.path, '/forum/comments/comment-1/revisions');
      return jsonResponse(<String, Object?>{
        'items': <Object?>[],
        'nextCursor': null,
        'hasMore': false,
      });
    });
    dio.httpClientAdapter = adapter;
    final ForumRepository repository = ForumRepository(ForumApi(dio));

    final ForumPageSlice<PostRevision> threadPage = await repository
        .threadRevisions('thread-1', cursor: 'thread-cursor');
    final ForumPageSlice<PostRevision> commentPage = await repository
        .commentRevisions('comment-1', cursor: 'comment-cursor');

    expect(threadPage.items.single.oldBody, '**旧正文**');
    expect(threadPage.nextCursor, 'next-page');
    expect(threadPage.hasMore, isTrue);
    expect(commentPage.items, isEmpty);
    expect(adapter.requests[0].uri.queryParameters, <String, String>{
      'cursor': 'thread-cursor',
      'limit': '20',
    });
    expect(adapter.requests[1].uri.queryParameters, <String, String>{
      'cursor': 'comment-cursor',
      'limit': '20',
    });
  });
}

Map<String, Object?> _revisionJson({required String id}) => <String, Object?>{
  'id': id,
  'seq': 2,
  'editorId': 'account-1',
  'oldTitle': '旧标题',
  'oldBody': '**旧正文**',
  'oldContentFormat': 'markdown_v1',
  'oldContentVersion': 1,
  'attachments': <Object?>[],
  'createdAt': 100,
};
