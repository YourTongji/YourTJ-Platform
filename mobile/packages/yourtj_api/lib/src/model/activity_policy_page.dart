//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/activity_policy.dart';
import 'package:json_annotation/json_annotation.dart';

part 'activity_policy_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ActivityPolicyPage {
  /// Returns a new [ActivityPolicyPage] instance.
  ActivityPolicyPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<ActivityPolicy> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ActivityPolicyPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory ActivityPolicyPage.fromJson(Map<String, dynamic> json) =>
      _$ActivityPolicyPageFromJson(json);

  Map<String, dynamic> toJson() => _$ActivityPolicyPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
