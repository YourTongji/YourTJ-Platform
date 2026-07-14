import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/messages/data/messages_repository.dart';

import '../auth/support/session_test_support.dart';

void main() {
  test(
    'conversation creation keeps one idempotency key and typed request body',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        expect(options.path, '/forum/dm/conversations');
        return jsonResponse(_conversationJson());
      });
      dio.httpClientAdapter = adapter;
      final MessagesRepository repository = MessagesRepository(ForumApi(dio));

      final DmConversation result = await repository.start(
        recipientHandle: ' alice ',
        requestMessage: '  想请教课程  ',
        idempotencyKey: 'conversation-key-1',
      );

      expect(result.requestStatus, DmConversationRequestStatusEnum.pending);
      final RecordedRequest request = adapter.requests.single;
      expect(request.headers['Idempotency-Key'], 'conversation-key-1');
      expect(requestJson(request), <String, Object?>{
        'recipientHandle': 'alice',
        'requestMessage': '想请教课程',
      });
    },
  );

  test(
    'conversation search sends the canonical server view and bounded query',
    () async {
      late final RecordingAdapter adapter;
      final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
      adapter = RecordingAdapter((RequestOptions options) {
        return jsonResponse(<String, Object?>{
          'items': <Object?>[],
          'nextCursor': null,
          'hasMore': false,
        });
      });
      dio.httpClientAdapter = adapter;
      final MessagesRepository repository = MessagesRepository(ForumApi(dio));

      await repository.conversations(
        view: ConversationView.archived,
        query: '  alice  ',
      );

      final RecordedRequest request = adapter.requests.single;
      expect(request.uri.queryParameters['view'], 'archived');
      expect(request.uri.queryParameters['q'], 'alice');
      expect(request.uri.queryParameters['limit'], '20');
    },
  );
}

Map<String, Object?> _conversationJson() => <String, Object?>{
  'id': 'conversation-1',
  'participantId': '2',
  'participantHandle': 'alice',
  'participantDisplayName': 'Alice',
  'participantAvatarUrl': null,
  'lastMessageExcerpt': '想请教课程',
  'lastMessageAt': 100,
  'unreadCount': 0,
  'isArchived': false,
  'isMuted': false,
  'isDeleted': false,
  'requestStatus': 'pending',
  'requestDirection': 'outgoing',
  'canSend': false,
  'createdAt': 100,
};
