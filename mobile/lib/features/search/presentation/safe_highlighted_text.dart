import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

class SafeHighlightedText extends StatelessWidget {
  const SafeHighlightedText({
    required this.text,
    this.ranges = const <SearchHighlightRange>[],
    this.style,
    this.maxLines,
    this.overflow = TextOverflow.clip,
    super.key,
  });

  final String text;
  final List<SearchHighlightRange> ranges;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow overflow;

  @override
  Widget build(BuildContext context) {
    final List<String> characters = text.runes
        .map(String.fromCharCode)
        .toList(growable: false);
    final List<SearchHighlightRange> normalized = normalizeHighlightRanges(
      ranges,
      characterCount: characters.length,
    );
    if (normalized.isEmpty) {
      return Text(text, style: style, maxLines: maxLines, overflow: overflow);
    }
    final List<InlineSpan> spans = <InlineSpan>[];
    int cursor = 0;
    for (final SearchHighlightRange range in normalized) {
      if (range.start > cursor) {
        spans.add(
          TextSpan(text: characters.sublist(cursor, range.start).join()),
        );
      }
      spans.add(
        TextSpan(
          text: characters.sublist(range.start, range.end).join(),
          style: TextStyle(
            backgroundColor: Theme.of(
              context,
            ).colorScheme.primary.withValues(alpha: 0.15),
          ),
        ),
      );
      cursor = range.end;
    }
    if (cursor < characters.length) {
      spans.add(TextSpan(text: characters.sublist(cursor).join()));
    }
    return Semantics(
      label: text,
      child: ExcludeSemantics(
        child: Text.rich(
          TextSpan(style: style, children: spans),
          maxLines: maxLines,
          overflow: overflow,
        ),
      ),
    );
  }
}

List<SearchHighlightRange> normalizeHighlightRanges(
  List<SearchHighlightRange> ranges, {
  required int characterCount,
}) {
  final List<SearchHighlightRange> candidates =
      ranges
          .where(
            (SearchHighlightRange range) =>
                range.start >= 0 &&
                range.end > range.start &&
                range.end <= characterCount,
          )
          .toList()
        ..sort((SearchHighlightRange left, SearchHighlightRange right) {
          final int byStart = left.start.compareTo(right.start);
          return byStart != 0 ? byStart : right.end.compareTo(left.end);
        });
  final List<SearchHighlightRange> normalized = <SearchHighlightRange>[];
  for (final SearchHighlightRange candidate in candidates) {
    if (normalized.isNotEmpty && candidate.start < normalized.last.end) {
      continue;
    }
    normalized.add(candidate);
    if (normalized.length == 8) {
      break;
    }
  }
  return normalized;
}
