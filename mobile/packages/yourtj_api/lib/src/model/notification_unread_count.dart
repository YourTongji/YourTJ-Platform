//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'notification_unread_count.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationUnreadCount {
  /// Returns a new [NotificationUnreadCount] instance.
  NotificationUnreadCount({required this.count});

  // minimum: 0
  @JsonKey(name: r'count', required: true, includeIfNull: false)
  final int count;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationUnreadCount && other.count == count;

  @override
  int get hashCode => count.hashCode;

  factory NotificationUnreadCount.fromJson(Map<String, dynamic> json) =>
      _$NotificationUnreadCountFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationUnreadCountToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
