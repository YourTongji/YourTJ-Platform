import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/content/platform_content.dart';

void main() {
  late List<Map<String, Object?>> cases;

  setUpAll(() async {
    final String source = await File(
      '../contract/fixtures/content-rendering-v1.json',
    ).readAsString();
    final Object? decoded = jsonDecode(source);
    final Map<String, Object?> corpus = (decoded as Map)
        .cast<String, Object?>();
    cases = (corpus['cases']! as List<Object?>)
        .map((Object? value) => (value! as Map).cast<String, Object?>())
        .toList(growable: false);
  });

  for (final String caseId in <String>[
    'plain-markup-remains-literal',
    'markdown-gfm-and-safe-links',
    'markdown-raw-html-is-rejected-and-skipped',
    'markdown-javascript-link-has-no-target',
    'markdown-data-link-has-no-target',
    'markdown-remote-image-never-loads',
    'markdown-data-image-never-loads',
    'markdown-platform-image-uses-derived-delivery',
    'markdown-noncanonical-asset-id-fails-closed',
    'markdown-malformed-link-remains-text',
    'markdown-overlong-source-is-not-canonical',
  ]) {
    testWidgets('conforms to shared corpus case $caseId', (
      WidgetTester tester,
    ) async {
      final Map<String, Object?> testCase = cases.singleWhere(
        (Map<String, Object?> value) => value['id'] == caseId,
      );
      String source = testCase['source']! as String;
      final Object? repeat = testCase['repeat'];
      if (repeat is int) {
        source = List<String>.filled(repeat, source).join();
      }
      final ContentFormat format = testCase['format'] == 'plain_v1'
          ? ContentFormat.plainV1
          : ContentFormat.markdownV1;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: PlatformContent(
                source: source,
                format: format,
                assetDeliveries: <int, Uri>{
                  42: Uri.parse('https://cdn.example.test/asset-42.webp'),
                },
                assetBuilder: (Uri delivery, String? label) {
                  return Text(
                    '已授权图片：$label',
                    key: ValueKey<String>(delivery.toString()),
                  );
                },
              ),
            ),
          ),
        ),
      );

      switch (caseId) {
        case 'plain-markup-remains-literal':
          expect(find.text(source), findsOneWidget);
          expect(find.byType(Image), findsNothing);
        case 'markdown-gfm-and-safe-links':
          expect(find.textContaining('规范标题'), findsWidgets);
          expect(find.textContaining('粗体'), findsWidgets);
          expect(find.byIcon(Icons.check_box), findsOneWidget);
        case 'markdown-raw-html-is-rejected-and-skipped':
          expect(find.textContaining('安全文本'), findsWidgets);
          expect(find.textContaining('window.evil'), findsNothing);
        case 'markdown-javascript-link-has-no-target' ||
            'markdown-data-link-has-no-target':
          expect(
            find.textContaining('危险链接'),
            caseId.startsWith('markdown-javascript')
                ? findsWidgets
                : findsNothing,
          );
          expect(
            find.textContaining('数据链接'),
            caseId.startsWith('markdown-data') ? findsWidgets : findsNothing,
          );
          expect(find.byType(AlertDialog), findsNothing);
        case 'markdown-remote-image-never-loads':
          expect(find.textContaining('图片当前不可用：跟踪像素'), findsOneWidget);
          expect(find.byType(Image), findsNothing);
        case 'markdown-data-image-never-loads':
          expect(find.textContaining('图片当前不可用：内联图片'), findsOneWidget);
          expect(find.byType(Image), findsNothing);
        case 'markdown-platform-image-uses-derived-delivery':
          expect(
            find.byKey(
              const ValueKey<String>('https://cdn.example.test/asset-42.webp'),
            ),
            findsOneWidget,
          );
        case 'markdown-noncanonical-asset-id-fails-closed':
          expect(find.textContaining('图片当前不可用：畸形资源'), findsOneWidget);
        case 'markdown-malformed-link-remains-text':
          expect(find.textContaining('[未闭合]'), findsWidgets);
        case 'markdown-overlong-source-is-not-canonical':
          expect(find.byType(SelectableText), findsOneWidget);
          final SelectableText text = tester.widget<SelectableText>(
            find.byType(SelectableText),
          );
          expect(text.data?.length, 50001);
      }
    });
  }
}
