//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/notification_outbox_event.dart';
import 'package:json_annotation/json_annotation.dart';

part 'notification_outbox_event_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationOutboxEventPage {
  /// Returns a new [NotificationOutboxEventPage] instance.
  NotificationOutboxEventPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<NotificationOutboxEvent> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationOutboxEventPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory NotificationOutboxEventPage.fromJson(Map<String, dynamic> json) =>
      _$NotificationOutboxEventPageFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationOutboxEventPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
