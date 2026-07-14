import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/search/presentation/safe_highlighted_text.dart';

void main() {
  test('normalizes bounded non-overlapping ranges', () {
    final List<SearchHighlightRange> normalized =
        normalizeHighlightRanges(<SearchHighlightRange>[
          SearchHighlightRange(start: 2, end: 4),
          SearchHighlightRange(start: 1, end: 3),
          SearchHighlightRange(start: -1, end: 1),
          SearchHighlightRange(start: 8, end: 10),
        ], characterCount: 9);

    expect(
      normalized.map((SearchHighlightRange range) => (range.start, range.end)),
      <(int, int)>[(1, 3)],
    );
  });

  testWidgets('renders Unicode offsets and markup-looking text as plain text', (
    WidgetTester tester,
  ) async {
    const String text = 'A😀算法<script>';
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SafeHighlightedText(
            text: text,
            ranges: <SearchHighlightRange>[
              SearchHighlightRange(start: 2, end: 4),
              SearchHighlightRange(start: 99, end: 100),
            ],
          ),
        ),
      ),
    );

    final RichText richText = tester.widget<RichText>(
      find.byType(RichText).last,
    );
    expect(richText.text.toPlainText(), text);
    expect(find.byType(HtmlElementView), findsNothing);
  });
}
