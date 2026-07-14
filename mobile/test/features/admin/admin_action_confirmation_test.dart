import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/admin/presentation/admin_action_confirmation.dart';

void main() {
  testWidgets('reason confirmation requires detail and explicit impact check', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: AdminReasonConfirmationDialog(
            actionLabel: '处理申诉',
            impact: '这会改变原管理决定。',
            expectedVersion: '7',
          ),
        ),
      ),
    );

    FilledButton submitButton = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '确认提交'),
    );
    expect(submitButton.onPressed, isNull);

    await tester.enterText(find.byType(TextField), '证据不足');
    await tester.tap(find.byType(Checkbox));
    await tester.pump();
    submitButton = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '确认提交'),
    );
    expect(submitButton.onPressed, isNull);

    await tester.enterText(find.byType(TextField), '复核证据后确认需要撤销原决定');
    await tester.pump();
    submitButton = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '确认提交'),
    );
    expect(submitButton.onPressed, isNotNull);
    expect(find.textContaining('409'), findsOneWidget);
    expect(find.textContaining('不可变审计'), findsOneWidget);
  });
}
