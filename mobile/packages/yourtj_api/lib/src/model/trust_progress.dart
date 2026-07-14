//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'trust_progress.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TrustProgress {
  /// Returns a new [TrustProgress] instance.
  TrustProgress({
    required this.trustLevel,

    required this.teaName,

    required this.qualifyingScore,

    required this.nextLevel,

    required this.nextThreshold,

    required this.remainingScore,

    required this.progressPercent,

    required this.policyVersion,

    required this.isMaxLevel,

    required this.overrideActive,

    required this.promotionBlockedUntil,

    required this.promotionRequiresNewActivity,
  });

  // minimum: 1
  // maximum: 6
  @JsonKey(name: r'trustLevel', required: true, includeIfNull: false)
  final int trustLevel;

  @JsonKey(name: r'teaName', required: true, includeIfNull: false)
  final String teaName;

  // minimum: 0
  @JsonKey(name: r'qualifyingScore', required: true, includeIfNull: false)
  final int qualifyingScore;

  // minimum: 2
  // maximum: 6
  @JsonKey(name: r'nextLevel', required: true, includeIfNull: true)
  final int? nextLevel;

  // minimum: 1
  @JsonKey(name: r'nextThreshold', required: true, includeIfNull: true)
  final int? nextThreshold;

  // minimum: 0
  @JsonKey(name: r'remainingScore', required: true, includeIfNull: true)
  final int? remainingScore;

  // minimum: 0
  // maximum: 100
  @JsonKey(name: r'progressPercent', required: true, includeIfNull: false)
  final int progressPercent;

  // minimum: 1
  @JsonKey(name: r'policyVersion', required: true, includeIfNull: false)
  final int policyVersion;

  @JsonKey(name: r'isMaxLevel', required: true, includeIfNull: false)
  final bool isMaxLevel;

  @JsonKey(name: r'overrideActive', required: true, includeIfNull: false)
  final bool overrideActive;

  @JsonKey(name: r'promotionBlockedUntil', required: true, includeIfNull: true)
  final int? promotionBlockedUntil;

  @JsonKey(
    name: r'promotionRequiresNewActivity',
    required: true,
    includeIfNull: false,
  )
  final bool promotionRequiresNewActivity;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TrustProgress &&
          other.trustLevel == trustLevel &&
          other.teaName == teaName &&
          other.qualifyingScore == qualifyingScore &&
          other.nextLevel == nextLevel &&
          other.nextThreshold == nextThreshold &&
          other.remainingScore == remainingScore &&
          other.progressPercent == progressPercent &&
          other.policyVersion == policyVersion &&
          other.isMaxLevel == isMaxLevel &&
          other.overrideActive == overrideActive &&
          other.promotionBlockedUntil == promotionBlockedUntil &&
          other.promotionRequiresNewActivity == promotionRequiresNewActivity;

  @override
  int get hashCode =>
      trustLevel.hashCode +
      teaName.hashCode +
      qualifyingScore.hashCode +
      (nextLevel == null ? 0 : nextLevel.hashCode) +
      (nextThreshold == null ? 0 : nextThreshold.hashCode) +
      (remainingScore == null ? 0 : remainingScore.hashCode) +
      progressPercent.hashCode +
      policyVersion.hashCode +
      isMaxLevel.hashCode +
      overrideActive.hashCode +
      (promotionBlockedUntil == null ? 0 : promotionBlockedUntil.hashCode) +
      promotionRequiresNewActivity.hashCode;

  factory TrustProgress.fromJson(Map<String, dynamic> json) =>
      _$TrustProgressFromJson(json);

  Map<String, dynamic> toJson() => _$TrustProgressToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
