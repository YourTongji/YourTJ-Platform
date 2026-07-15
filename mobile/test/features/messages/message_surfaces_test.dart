import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/appeals/presentation/appeals_page.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/messages/data/messages_repository.dart';
import 'package:yourtj_mobile/features/messages/presentation/direct_messages_page.dart';
import 'package:yourtj_mobile/features/messages/presentation/message_dialogs.dart';
import 'package:yourtj_mobile/features/notifications/presentation/notifications_page.dart';

void main() {
  testWidgets('anonymous notification surface offers a login recovery path', (
    WidgetTester tester,
  ) async {
    await _pumpAnonymous(tester, const NotificationsPage(embedded: true));

    expect(find.text('登录后查看通知'), findsOneWidget);
    expect(find.text('登录'), findsOneWidget);
  });

  testWidgets(
    'anonymous direct-message surface does not expose cached content',
    (WidgetTester tester) async {
      await _pumpAnonymous(
        tester,
        const DirectMessagesPage(initialView: ConversationView.inbox),
      );

      expect(find.text('登录后查看私信'), findsOneWidget);
      expect(find.textContaining('不会写入普通本地缓存'), findsOneWidget);
    },
  );

  testWidgets('restricted appeal entry explains its narrow credential scope', (
    WidgetTester tester,
  ) async {
    await _pumpAnonymous(tester, const AppealsPage(initialEventId: '44'));

    expect(find.text('安全进入申诉中心'), findsOneWidget);
    expect(find.textContaining('不能访问资料、内容、私信或积分'), findsOneWidget);
    expect(find.text('同济校园邮箱'), findsOneWidget);
  });

  testWidgets('new conversation accepts only the OpenAPI handle format', (
    WidgetTester tester,
  ) async {
    NewConversationDraft? result;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.light,
        home: Builder(
          builder: (BuildContext context) => Scaffold(
            body: FilledButton(
              onPressed: () async {
                result = await showNewConversationDialog(context);
              },
              child: const Text('发起'),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('发起'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField).first, 'Student');
    await tester.tap(find.text('继续'));
    await tester.pump();
    expect(find.textContaining('小写字母'), findsOneWidget);

    await tester.enterText(
      find.byType(TextFormField).first,
      'student.name-test_2026',
    );
    await tester.tap(find.text('继续'));
    await tester.pumpAndSettle();

    expect(result?.handle, 'student.name-test_2026');
  });

  testWidgets('message report keeps its editor alive through route exit', (
    WidgetTester tester,
  ) async {
    DmReportDraft? result;
    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.light,
        home: Builder(
          builder: (BuildContext context) => Scaffold(
            body: FilledButton(
              onPressed: () async {
                result = await showDmReportDialog(context, isRequest: false);
              },
              child: const Text('举报'),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('举报'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).last, '需要审核');
    await tester.tap(find.text('提交举报'));
    await tester.pumpAndSettle();

    expect(result?.reason, DmReportInputReasonEnum.spam);
    expect(result?.note, '需要审核');
    expect(tester.takeException(), isNull);
  });

  testWidgets('account change invalidates an in-flight conversation start', (
    WidgetTester tester,
  ) async {
    final StreamController<SessionState> sessions =
        StreamController<SessionState>();
    addTearDown(sessions.close);
    final _DelayedMessagesRepository repository = _DelayedMessagesRepository();
    sessions.add(
      SessionState.authenticated(generation: 1, account: _account('account-a')),
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          sessionStateProvider.overrideWith((Ref ref) => sessions.stream),
          messagesRepositoryProvider.overrideWithValue(repository),
        ],
        child: MaterialApp(
          theme: AppTheme.light,
          home: const Scaffold(
            body: DirectMessagesPage(initialView: ConversationView.inbox),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('新私信'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField).first, 'old-user');
    await tester.tap(find.text('继续'));
    await tester.pump();
    expect(repository.startCalls, 1);

    sessions.add(
      SessionState.authenticated(generation: 2, account: _account('account-b')),
    );
    await tester.pumpAndSettle();
    final FilledButton newMessageButton = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '新私信'),
    );
    expect(newMessageButton.onPressed, isNotNull);

    repository.startResult.complete(_conversation());
    await tester.pumpAndSettle();

    expect(find.textContaining('已打开与 @old-user'), findsNothing);
    expect(find.textContaining('消息请求已发送'), findsNothing);
  });
}

Future<void> _pumpAnonymous(WidgetTester tester, Widget child) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        sessionStateProvider.overrideWith(
          (Ref ref) => Stream<SessionState>.value(
            const SessionState.anonymous(generation: 1),
          ),
        ),
      ],
      child: MaterialApp(
        theme: AppTheme.light,
        home: Scaffold(body: child),
      ),
    ),
  );
  await tester.pumpAndSettle();
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

DmConversation _conversation() {
  return DmConversation(
    id: 'conversation-a',
    participantId: 'recipient-a',
    participantHandle: 'old-user',
    unreadCount: 0,
    isArchived: false,
    isMuted: false,
    isDeleted: false,
    requestStatus: DmConversationRequestStatusEnum.pending,
    requestDirection: DmConversationRequestDirectionEnum.outgoing,
    canSend: false,
    createdAt: 1,
  );
}

class _DelayedMessagesRepository extends MessagesRepository {
  _DelayedMessagesRepository() : super(ForumApi(Dio()));

  final Completer<DmConversation> startResult = Completer<DmConversation>();
  int startCalls = 0;

  @override
  Future<DmConversationPage> conversations({
    required ConversationView view,
    String? query,
    String? cursor,
  }) async {
    return DmConversationPage(
      items: const <DmConversation>[],
      nextCursor: null,
      hasMore: false,
    );
  }

  @override
  Future<DmCounts> counts() async {
    return DmCounts(count: 0, unreadCount: 0, requestCount: 0);
  }

  @override
  Future<DmConversation> start({
    required String recipientHandle,
    required String requestMessage,
    required String idempotencyKey,
  }) {
    startCalls += 1;
    return startResult.future;
  }
}
