//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'trust_level_policy.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TrustLevelPolicy {
  /// Returns a new [TrustLevelPolicy] instance.
  TrustLevelPolicy({
    required this.version,

    required this.scorePolicyVersion,

    required this.thresholdLevel2,

    required this.thresholdLevel3,

    required this.thresholdLevel4,

    required this.thresholdLevel5,

    required this.thresholdLevel6,

    required this.likeDailyCap,

    required this.demotionCooldownDays,

    required this.reason,

    required this.changedBy,

    required this.createdAt,
  });

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  // minimum: 1
  @JsonKey(name: r'scorePolicyVersion', required: true, includeIfNull: false)
  final int scorePolicyVersion;

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

  @JsonKey(name: r'changedBy', required: true, includeIfNull: false)
  final String changedBy;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrustLevelPolicy &&
          other.version == version &&
          other.scorePolicyVersion == scorePolicyVersion &&
          other.thresholdLevel2 == thresholdLevel2 &&
          other.thresholdLevel3 == thresholdLevel3 &&
          other.thresholdLevel4 == thresholdLevel4 &&
          other.thresholdLevel5 == thresholdLevel5 &&
          other.thresholdLevel6 == thresholdLevel6 &&
          other.likeDailyCap == likeDailyCap &&
          other.demotionCooldownDays == demotionCooldownDays &&
          other.reason == reason &&
          other.changedBy == changedBy &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      version.hashCode +
      scorePolicyVersion.hashCode +
      thresholdLevel2.hashCode +
      thresholdLevel3.hashCode +
      thresholdLevel4.hashCode +
      thresholdLevel5.hashCode +
      thresholdLevel6.hashCode +
      likeDailyCap.hashCode +
      demotionCooldownDays.hashCode +
      reason.hashCode +
      changedBy.hashCode +
      createdAt.hashCode;

  factory TrustLevelPolicy.fromJson(Map<String, dynamic> json) =>
      _$TrustLevelPolicyFromJson(json);

  Map<String, dynamic> toJson() => _$TrustLevelPolicyToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
