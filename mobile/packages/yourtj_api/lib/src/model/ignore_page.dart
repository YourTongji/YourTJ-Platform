//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/ignore_user.dart';
import 'package:json_annotation/json_annotation.dart';

part 'ignore_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class IgnorePage {
  /// Returns a new [IgnorePage] instance.
  IgnorePage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<IgnoreUser> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is IgnorePage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory IgnorePage.fromJson(Map<String, dynamic> json) =>
      _$IgnorePageFromJson(json);

  Map<String, dynamic> toJson() => _$IgnorePageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
