//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/achievement_grant.dart';
import 'package:json_annotation/json_annotation.dart';

part 'achievement_grant_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementGrantPage {
  /// Returns a new [AchievementGrantPage] instance.
  AchievementGrantPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<AchievementGrant> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementGrantPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory AchievementGrantPage.fromJson(Map<String, dynamic> json) =>
      _$AchievementGrantPageFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementGrantPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
