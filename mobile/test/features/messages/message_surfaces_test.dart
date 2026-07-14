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
