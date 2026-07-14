import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/admin/domain/admin_mutations.dart';
import 'package:yourtj_mobile/features/admin/presentation/admin_mutation_dialog.dart';

void main() {
  testWidgets(
    'mutation confirmation requires fields, detailed reason, and impact check',
    (WidgetTester tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: AdminMutationDialog(
              action: AdminMutationAction(
                kind: AdminMutationKind.decideAppeal,
                label: '裁决申诉',
                impact: '这会改变原管理决定。',
                requiredAnyCapability: <String>{'appeals.review'},
                expectedVersion: 7,
                fields: <AdminMutationField>[
                  AdminMutationField(
                    key: 'outcome',
                    label: '申诉结果',
                    kind: AdminMutationFieldKind.choice,
                    options: <AdminMutationOption>[
                      AdminMutationOption('upheld', '维持'),
                      AdminMutationOption('overturned', '推翻'),
                    ],
                  ),
                  AdminMutationField(
                    key: 'targetId',
                    label: '目标 ID',
                    isRequired: true,
                  ),
                ],
              ),
            ),
          ),
        ),
      );

      FilledButton submit = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, '确认提交'),
      );
      expect(submit.onPressed, isNull);
      expect(find.textContaining('已审阅版本：7'), findsOneWidget);
      expect(find.textContaining('HTTP 409'), findsOneWidget);
      expect(find.textContaining('不会自动重试'), findsOneWidget);

      await tester.enterText(
        find.widgetWithText(TextField, '目标 ID'),
        'appeal-1',
      );
      await tester.enterText(
        find.widgetWithText(TextField, '操作理由'),
        '复核完整证据后确认需要推翻原决定',
      );
      await tester.tap(find.byType(Checkbox));
      await tester.pump();

      submit = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, '确认提交'),
      );
      expect(submit.onPressed, isNotNull);
    },
  );

  testWidgets('mandatory safety acknowledgement cannot submit as false', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: AdminMutationDialog(
            action: AdminMutationAction(
              kind: AdminMutationKind.approveMedia,
              label: '批准本人媒体',
              impact: '媒体将进入公开交付处理。',
              requiredAnyCapability: <String>{'moderation.content'},
              fields: <AdminMutationField>[
                AdminMutationField(
                  key: 'selfReviewConfirmed',
                  label: '确认本人媒体例外',
                  kind: AdminMutationFieldKind.boolean,
                  mustBeTrue: true,
                ),
              ],
            ),
          ),
        ),
      ),
    );

    await tester.enterText(
      find.widgetWithText(TextField, '操作理由'),
      '复核可信预览后确认适用管理员本人媒体例外',
    );
    await tester.tap(find.byType(Checkbox));
    await tester.pump();
    FilledButton submit = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '确认提交'),
    );
    expect(submit.onPressed, isNull);

    await tester.tap(find.byType(Switch));
    await tester.pump();
    submit = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '确认提交'),
    );
    expect(submit.onPressed, isNotNull);
  });
}
