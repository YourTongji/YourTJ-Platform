//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/user_summary.dart';
import 'package:json_annotation/json_annotation.dart';

part 'user_summary_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserSummaryPage {
  /// Returns a new [UserSummaryPage] instance.
  UserSummaryPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<UserSummary> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserSummaryPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory UserSummaryPage.fromJson(Map<String, dynamic> json) =>
      _$UserSummaryPageFromJson(json);

  Map<String, dynamic> toJson() => _$UserSummaryPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
