//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/verification_type.dart';
import 'package:json_annotation/json_annotation.dart';

part 'verification_type_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VerificationTypePage {
  /// Returns a new [VerificationTypePage] instance.
  VerificationTypePage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<VerificationType> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VerificationTypePage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory VerificationTypePage.fromJson(Map<String, dynamic> json) =>
      _$VerificationTypePageFromJson(json);

  Map<String, dynamic> toJson() => _$VerificationTypePageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
