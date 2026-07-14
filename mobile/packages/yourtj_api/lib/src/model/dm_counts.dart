//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_counts.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmCounts {
  /// Returns a new [DmCounts] instance.
  DmCounts({
    required this.count,

    required this.unreadCount,

    required this.requestCount,
  });

  /// unreadCount plus requestCount for compatibility badges.
  // minimum: 0
  @JsonKey(name: r'count', required: true, includeIfNull: false)
  final int count;

  // minimum: 0
  @JsonKey(name: r'unreadCount', required: true, includeIfNull: false)
  final int unreadCount;

  // minimum: 0
  @JsonKey(name: r'requestCount', required: true, includeIfNull: false)
  final int requestCount;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmCounts &&
          other.count == count &&
          other.unreadCount == unreadCount &&
          other.requestCount == requestCount;

  @override
  int get hashCode =>
      count.hashCode + unreadCount.hashCode + requestCount.hashCode;

  factory DmCounts.fromJson(Map<String, dynamic> json) =>
      _$DmCountsFromJson(json);

  Map<String, dynamic> toJson() => _$DmCountsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
