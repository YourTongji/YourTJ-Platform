//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'trust_level_policy_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TrustLevelPolicyUpdateInput {
  /// Returns a new [TrustLevelPolicyUpdateInput] instance.
  TrustLevelPolicyUpdateInput({
    required this.expectedVersion,

    required this.thresholdLevel2,

    required this.thresholdLevel3,

    required this.thresholdLevel4,

    required this.thresholdLevel5,

    required this.thresholdLevel6,

    required this.likeDailyCap,

    required this.demotionCooldownDays,

    required this.reason,
  });

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  // minimum: 1
  @JsonKey(name: r'thresholdLevel2', required: true, includeIfNull: false)
  final int thresholdLevel2;

  // minimum: 1
  @JsonKey(name: r'thresholdLevel3', required: true, includeIfNull: false)
  final int thresholdLevel3;

  // minimum: 1
  @JsonKey(name: r'thresholdLevel4', required: true, includeIfNull: false)
  final int thresholdLevel4;

  // minimum: 1
  @JsonKey(name: r'thresholdLevel5', required: true, includeIfNull: false)
  final int thresholdLevel5;

  // minimum: 1
  @JsonKey(name: r'thresholdLevel6', required: true, includeIfNull: false)
  final int thresholdLevel6;

  // minimum: 0
  // maximum: 100000
  @JsonKey(name: r'likeDailyCap', required: true, includeIfNull: false)
  final int likeDailyCap;

  // minimum: 0
  // maximum: 365
  @JsonKey(name: r'demotionCooldownDays', required: true, includeIfNull: false)
  final int demotionCooldownDays;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrustLevelPolicyUpdateInput &&
          other.expectedVersion == expectedVersion &&
          other.thresholdLevel2 == thresholdLevel2 &&
          other.thresholdLevel3 == thresholdLevel3 &&
          other.thresholdLevel4 == thresholdLevel4 &&
          other.thresholdLevel5 == thresholdLevel5 &&
          other.thresholdLevel6 == thresholdLevel6 &&
          other.likeDailyCap == likeDailyCap &&
          other.demotionCooldownDays == demotionCooldownDays &&
          other.reason == reason;

  @override
  int get hashCode =>
      expectedVersion.hashCode +
      thresholdLevel2.hashCode +
      thresholdLevel3.hashCode +
      thresholdLevel4.hashCode +
      thresholdLevel5.hashCode +
      thresholdLevel6.hashCode +
      likeDailyCap.hashCode +
      demotionCooldownDays.hashCode +
      reason.hashCode;

  factory TrustLevelPolicyUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$TrustLevelPolicyUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$TrustLevelPolicyUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
