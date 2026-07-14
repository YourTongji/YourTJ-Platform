import 'package:json_annotation/json_annotation.dart';
import 'package:test/test.dart';
import 'package:yourtj_api/yourtj_api.dart';

void main() {
  group('forum draft discriminator', () {
    test('round-trips a thread draft through its concrete model', () {
      final wireJson = <String, dynamic>{
        'draftKey': 'thread:new',
        'expectedVersion': 0,
        'payload': <String, dynamic>{
          'kind': 'thread',
          'boardId': '1',
          'title': '选课建议',
          'body': '正文',
          'contentFormat': 'markdown_v1',
          'tags': <String>['选课'],
          'pollQuestion': '',
          'pollOptions': <String>[],
          'attachmentAssetIds': <String>['101'],
        },
      };

      final draft = DraftSaveInput.fromJson(wireJson);

      expect(draft.payload, isA<ForumThreadDraftPayload>());
      final payload = draft.payload as ForumThreadDraftPayload;
      expect(payload.payload.boardId, '1');
      expect(payload.payload.kind, ThreadDraftPayloadKindEnum.thread);
      expect(draft.toJson(), equals(wireJson));
    });

    test('round-trips a comment draft through its concrete model', () {
      final wireJson = <String, dynamic>{
        'draftKey': 'comment:1',
        'expectedVersion': 3,
        'payload': <String, dynamic>{
          'kind': 'comment',
          'threadId': '1',
          'body': '回复正文',
          'contentFormat': 'plain_v1',
          'parentId': null,
          'attachmentAssetIds': <String>[],
        },
      };

      final draft = DraftSaveInput.fromJson(wireJson);

      expect(draft.payload, isA<ForumCommentDraftPayload>());
      final payload = draft.payload as ForumCommentDraftPayload;
      expect(payload.payload.threadId, '1');
      expect(payload.payload.kind, CommentDraftPayloadKindEnum.comment);
      expect(draft.toJson(), equals(wireJson));
    });

    test('rejects a missing or unknown discriminator', () {
      for (final wireJson in <Map<String, dynamic>>[
        <String, dynamic>{},
        <String, dynamic>{'kind': 'poll'},
      ]) {
        expect(
          () => ForumDraftPayload.fromJson(wireJson),
          throwsA(isA<CheckedFromJsonException>()),
        );
      }
    });
  });

  group('required response fields', () {
    test('requires every cursor page envelope field', () {
      final completePage = <String, dynamic>{
        'items': <Object>[],
        'nextCursor': null,
        'hasMore': false,
      };

      final page = CoursePage.fromJson(completePage);
      expect(page.items, isEmpty);
      expect(page.nextCursor, isNull);
      expect(page.hasMore, isFalse);

      for (final requiredField in <String>['items', 'nextCursor', 'hasMore']) {
        final incompletePage = Map<String, dynamic>.from(completePage)
          ..remove(requiredField);
        expect(
          () => CoursePage.fromJson(incompletePage),
          throwsA(isA<CheckedFromJsonException>()),
          reason: '$requiredField must remain required',
        );
      }
    });

    test('requires both wallet claim challenge fields', () {
      final challenge = WalletClaimChallenge.fromJson(<String, dynamic>{
        'challengeId': 'challenge-1',
        'nonce': 'nonce-1',
      });
      expect(challenge.challengeId, 'challenge-1');
      expect(challenge.nonce, 'nonce-1');

      for (final requiredField in <String>['challengeId', 'nonce']) {
        final incompleteChallenge = <String, dynamic>{
          'challengeId': 'challenge-1',
          'nonce': 'nonce-1',
        }..remove(requiredField);
        expect(
          () => WalletClaimChallenge.fromJson(incompleteChallenge),
          throwsA(isA<CheckedFromJsonException>()),
          reason: '$requiredField must remain required',
        );
      }
    });

    test('requires the named notification unread count', () {
      final unreadCount = NotificationUnreadCount.fromJson(<String, dynamic>{
        'count': 7,
      });
      expect(unreadCount.count, 7);

      expect(
        () => NotificationUnreadCount.fromJson(<String, dynamic>{}),
        throwsA(isA<CheckedFromJsonException>()),
      );
    });
  });
}
