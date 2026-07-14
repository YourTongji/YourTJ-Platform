//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/thread_feed.dart';
import 'package:json_annotation/json_annotation.dart';

part 'thread_feed_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadFeedPage {
  /// Returns a new [ThreadFeedPage] instance.
  ThreadFeedPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<ThreadFeed> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadFeedPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory ThreadFeedPage.fromJson(Map<String, dynamic> json) =>
      _$ThreadFeedPageFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadFeedPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
