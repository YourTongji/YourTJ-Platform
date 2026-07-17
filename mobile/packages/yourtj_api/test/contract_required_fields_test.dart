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

    test(
      'requires the complete wallet owner projection including explicit key state',
      () {
        final completeWallet = <String, dynamic>{
          'accountId': 'account-1',
          'balance': 100,
          'activePublicKey': null,
        };

        final wallet = Wallet.fromJson(completeWallet);
        expect(wallet.accountId, 'account-1');
        expect(wallet.balance, 100);
        expect(wallet.activePublicKey, isNull);

        for (final requiredField in <String>[
          'accountId',
          'balance',
          'activePublicKey',
        ]) {
          final incompleteWallet = Map<String, dynamic>.from(completeWallet)
            ..remove(requiredField);
          expect(
            () => Wallet.fromJson(incompleteWallet),
            throwsA(isA<CheckedFromJsonException>()),
            reason: '$requiredField must remain required',
          );
        }
      },
    );

    test('requires the complete task projection including explicit nulls', () {
      final completeTask = <String, dynamic>{
        'id': '7',
        'creatorId': '1',
        'acceptorId': null,
        'title': '取快递',
        'description': null,
        'rewardAmount': 10,
        'contactInfo': null,
        'status': 'open',
        'createdAt': 2000000000,
      };

      final task = Task.fromJson(completeTask);
      expect(task.id, '7');
      expect(task.acceptorId, isNull);
      expect(task.description, isNull);
      expect(task.contactInfo, isNull);

      for (final requiredField in completeTask.keys) {
        final incompleteTask = Map<String, dynamic>.from(completeTask)
          ..remove(requiredField);
        expect(
          () => Task.fromJson(incompleteTask),
          throwsA(isA<CheckedFromJsonException>()),
          reason: '$requiredField must remain required',
        );
      }
    });

    test('requires the complete product projection including description', () {
      final completeProduct = <String, dynamic>{
        'id': '8',
        'sellerId': '2',
        'title': '二手教材',
        'description': null,
        'price': 20,
        'stock': 1,
        'status': 'on_sale',
        'createdAt': 2000000000,
      };

      final product = Product.fromJson(completeProduct);
      expect(product.id, '8');
      expect(product.description, isNull);

      for (final requiredField in completeProduct.keys) {
        final incompleteProduct = Map<String, dynamic>.from(completeProduct)
          ..remove(requiredField);
        expect(
          () => Product.fromJson(incompleteProduct),
          throwsA(isA<CheckedFromJsonException>()),
          reason: '$requiredField must remain required',
        );
      }
    });

    test(
      'requires the complete purchase projection including delivery state',
      () {
        final completePurchase = <String, dynamic>{
          'id': '9',
          'productId': '8',
          'buyerId': '1',
          'sellerId': '2',
          'amount': 20,
          'status': 'pending',
          'deliveryInfo': null,
          'createdAt': 2000000000,
        };

        final purchase = Purchase.fromJson(completePurchase);
        expect(purchase.id, '9');
        expect(purchase.deliveryInfo, isNull);

        for (final requiredField in completePurchase.keys) {
          final incompletePurchase = Map<String, dynamic>.from(completePurchase)
            ..remove(requiredField);
          expect(
            () => Purchase.fromJson(incompletePurchase),
            throwsA(isA<CheckedFromJsonException>()),
            reason: '$requiredField must remain required',
          );
        }
      },
    );

    test('requires the complete signing intent outcome', () {
      final completeOutcome = <String, dynamic>{
        'intentId': 'intent-1',
        'status': 'pending',
        'expiresAt': 2000000000,
      };

      final outcome = SigningIntentOutcome.fromJson(completeOutcome);
      expect(outcome.intentId, 'intent-1');
      expect(outcome.status, SigningIntentOutcomeStatusEnum.pending);
      expect(outcome.expiresAt, 2000000000);

      for (final requiredField in <String>['intentId', 'status', 'expiresAt']) {
        final incompleteOutcome = Map<String, dynamic>.from(completeOutcome)
          ..remove(requiredField);
        expect(
          () => SigningIntentOutcome.fromJson(incompleteOutcome),
          throwsA(isA<CheckedFromJsonException>()),
          reason: '$requiredField must remain required',
        );
      }
    });

    test('requires the signing intent outcome correlation body', () {
      final input = SigningIntentOutcomeInput.fromJson(<String, dynamic>{
        'intentId': 'intent-1',
      });
      expect(input.intentId, 'intent-1');

      expect(
        () => SigningIntentOutcomeInput.fromJson(<String, dynamic>{}),
        throwsA(isA<CheckedFromJsonException>()),
      );
    });

    test(
      'keeps account scope optional only for legacy wallet confirmation',
      () {
        final completeRequest = <String, dynamic>{
          'accountId': 'account-1',
          'publicKey': 'public-key',
        };

        final request = WalletBindPostRequest.fromJson(completeRequest);
        expect(request.accountId, 'account-1');
        expect(request.publicKey, 'public-key');

        final legacyConfirmation = WalletBindPostRequest.fromJson(
          <String, dynamic>{'publicKey': 'public-key'},
        );
        expect(legacyConfirmation.accountId, isNull);

        expect(
          () => WalletBindPostRequest.fromJson(<String, dynamic>{
            'accountId': 'account-1',
          }),
          throwsA(isA<CheckedFromJsonException>()),
          reason: 'publicKey must remain required',
        );
      },
    );

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
