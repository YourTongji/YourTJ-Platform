import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/forum/presentation/forum_markdown_composer.dart';

void main() {
  testWidgets('preview uses the final safe renderer and exposes named tabs', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(
      text:
          '**安全预览**\n\n<script>不能出现</script>\n\n'
          '![远程图片](https://attacker.example/image.png)',
    );
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: AppTheme.light,
        home: Scaffold(
          body: SingleChildScrollView(
            child: ForumMarkdownComposer(
              controller: controller,
              label: '正文（Markdown）',
              maxLength: 50000,
              minLines: 5,
              maxLines: 12,
            ),
          ),
        ),
      ),
    );

    expect(find.text('编辑'), findsOneWidget);
    expect(find.text('预览'), findsOneWidget);
    await tester.tap(find.text('预览'));
    await tester.pump();

    expect(find.bySemanticsLabel('正文（Markdown） Markdown 预览'), findsOneWidget);
    expect(find.text('安全预览'), findsOneWidget);
    expect(find.textContaining('不能出现'), findsNothing);
    expect(find.textContaining('图片当前不可用：远程图片'), findsOneWidget);

    controller.clear();
    await tester.pump();

    expect(find.text('没有可预览的内容'), findsOneWidget);
  });
}
