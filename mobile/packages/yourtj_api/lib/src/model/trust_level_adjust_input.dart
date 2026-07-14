//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'trust_level_adjust_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TrustLevelAdjustInput {
  /// Returns a new [TrustLevelAdjustInput] instance.
  TrustLevelAdjustInput({
    this.trustLevel,

    this.clearOverride = false,

    required this.reason,
  });

  // minimum: 1
  // maximum: 6
  @JsonKey(name: r'trustLevel', required: false, includeIfNull: false)
  final int? trustLevel;

  @JsonKey(
    defaultValue: false,
    name: r'clearOverride',
    required: false,
    includeIfNull: false,
  )
  final bool? clearOverride;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrustLevelAdjustInput &&
          other.trustLevel == trustLevel &&
          other.clearOverride == clearOverride &&
          other.reason == reason;

  @override
  int get hashCode =>
      (trustLevel == null ? 0 : trustLevel.hashCode) +
      clearOverride.hashCode +
      reason.hashCode;

  factory TrustLevelAdjustInput.fromJson(Map<String, dynamic> json) =>
      _$TrustLevelAdjustInputFromJson(json);

  Map<String, dynamic> toJson() => _$TrustLevelAdjustInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
