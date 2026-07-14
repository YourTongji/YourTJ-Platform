import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/profile/data/profile_activity_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test('uses each owner API with stable cursor pagination', () async {
    late final RecordingAdapter adapter;
    final ProfileActivityRepository repository = _repository((
      RequestOptions options,
    ) {
      return switch (options.path) {
        '/users/alice/threads' => jsonResponse(_threadPage()),
        '/users/alice/comments' => jsonResponse(_commentPage()),
        '/users/alice/media' => jsonResponse(_contentPage('media-1')),
        '/users/alice/likes' => jsonResponse(_contentPage('liked-1')),
        _ => jsonResponse(<String, Object?>{}, statusCode: 404),
      };
    }, onAdapter: (RecordingAdapter value) => adapter = value);

    final ProfileActivityPage<UserThread> threads = await repository.threads(
      handle: 'alice',
      cursor: 'thread-cursor',
    );
    final ProfileActivityPage<UserComment> comments = await repository.comments(
      handle: 'alice',
      cursor: 'comment-cursor',
    );
    final ProfileActivityPage<ProfileContent> media = await repository.media(
      handle: 'alice',
      cursor: 'media-cursor',
    );
    final ProfileActivityPage<ProfileContent> likes = await repository.likes(
      handle: 'alice',
      cursor: 'likes-cursor',
    );

    expect(threads.items.single.id, 'thread-1');
    expect(comments.items.single.id, 'comment-1');
    expect(media.items.single.id, 'media-1');
    expect(likes.items.single.id, 'liked-1');
    expect(
      adapter.requests.map((RecordedRequest request) => request.uri.path),
      <String>[
        '/api/v2/users/alice/threads',
        '/api/v2/users/alice/comments',
        '/api/v2/users/alice/media',
        '/api/v2/users/alice/likes',
      ],
    );
    expect(
      adapter.requests.map(
        (RecordedRequest request) => request.uri.queryParameters['limit'],
      ),
      everyElement('20'),
    );
    expect(
      adapter.requests.map(
        (RecordedRequest request) => request.uri.queryParameters['cursor'],
      ),
      <String>[
        'thread-cursor',
        'comment-cursor',
        'media-cursor',
        'likes-cursor',
      ],
    );
  });

  test('rejects a has-more response without a continuation cursor', () async {
    final ProfileActivityRepository repository = _repository((
      RequestOptions options,
    ) {
      return jsonResponse(<String, Object?>{
        ..._threadPage(),
        'hasMore': true,
        'nextCursor': null,
      });
    });

    await expectLater(
      repository.threads(handle: 'alice'),
      throwsA(
        isA<ApiFailure>().having(
          (ApiFailure failure) => failure.kind,
          'kind',
          ApiFailureKind.unexpected,
        ),
      ),
    );
  });

  test('preserves permission failures from the owner API', () async {
    final ProfileActivityRepository repository = _repository((
      RequestOptions options,
    ) {
      return jsonResponse(<String, Object?>{
        'error': <String, String>{
          'code': 'PROFILE_ACTIVITY_PRIVATE',
          'message': '活动列表未公开',
        },
      }, statusCode: 403);
    });

    await expectLater(
      repository.likes(handle: 'alice'),
      throwsA(
        isA<ApiFailure>()
            .having(
              (ApiFailure failure) => failure.kind,
              'kind',
              ApiFailureKind.forbidden,
            )
            .having(
              (ApiFailure failure) => failure.code,
              'code',
              'PROFILE_ACTIVITY_PRIVATE',
            ),
      ),
    );
  });
}

ProfileActivityRepository _repository(
  AdapterHandler handler, {
  void Function(RecordingAdapter adapter)? onAdapter,
}) {
  final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
  final RecordingAdapter adapter = RecordingAdapter(handler);
  dio.httpClientAdapter = adapter;
  onAdapter?.call(adapter);
  return ProfileActivityRepository(IdentityApi(dio), ForumApi(dio));
}

Map<String, Object?> _threadPage() => <String, Object?>{
  'items': <Object?>[
    <String, Object?>{
      'id': 'thread-1',
      'title': '第一条主题',
      'bodyExcerpt': '主题摘要',
      'contentFormat': 'plain_v1',
      'boardSlug': 'campus',
      'replyCount': 2,
      'voteCount': 3,
      'viewerVote': null,
      'isBookmarked': false,
      'attachments': <Object?>[],
      'createdAt': 100,
    },
  ],
  'nextCursor': null,
  'hasMore': false,
};

Map<String, Object?> _commentPage() => <String, Object?>{
  'items': <Object?>[
    <String, Object?>{
      'id': 'comment-1',
      'threadId': 'thread-1',
      'threadTitle': '第一条主题',
      'body': '回复正文',
      'contentFormat': 'plain_v1',
      'replyCount': 1,
      'voteCount': 4,
      'viewerVote': null,
      'isBookmarked': false,
      'attachments': <Object?>[],
      'createdAt': 101,
    },
  ],
  'nextCursor': null,
  'hasMore': false,
};

Map<String, Object?> _contentPage(String id) => <String, Object?>{
  'items': <Object?>[
    <String, Object?>{
      'targetType': 'thread',
      'id': id,
      'threadId': id,
      'title': '动态内容',
      'body': '动态正文',
      'contentFormat': 'plain_v1',
      'boardSlug': 'campus',
      'authorHandle': 'alice',
      'authorDisplayName': 'Alice',
      'replyCount': 2,
      'voteCount': 5,
      'viewerVote': null,
      'isBookmarked': false,
      'attachments': <Object?>[],
      'createdAt': 102,
      'activityAt': 103,
    },
  ],
  'nextCursor': null,
  'hasMore': false,
};
