//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/achievement_icon.dart';
import 'package:yourtj_api/src/model/achievement_status.dart';
import 'package:json_annotation/json_annotation.dart';

part 'achievement_grant.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AchievementGrant {
  /// Returns a new [AchievementGrant] instance.
  AchievementGrant({
    required this.accountId,

    required this.achievementId,

    required this.slug,

    required this.name,

    required this.icon,

    required this.definitionStatus,

    required this.status,

    required this.awardReason,

    required this.awardedAt,

    required this.awardedBy,

    required this.revokedAt,

    required this.revokedBy,

    required this.revokeReason,
  });

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(name: r'achievementId', required: true, includeIfNull: false)
  final String achievementId;

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(
    name: r'icon',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementIcon.unknownDefaultOpenApi,
  )
  final AchievementIcon icon;

  @JsonKey(
    name: r'definitionStatus',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementStatus.unknownDefaultOpenApi,
  )
  final AchievementStatus definitionStatus;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AchievementGrantStatusEnum.unknownDefaultOpenApi,
  )
  final AchievementGrantStatusEnum status;

  @JsonKey(name: r'awardReason', required: true, includeIfNull: true)
  final String? awardReason;

  @JsonKey(name: r'awardedAt', required: true, includeIfNull: false)
  final int awardedAt;

  @JsonKey(name: r'awardedBy', required: true, includeIfNull: false)
  final String awardedBy;

  @JsonKey(name: r'revokedAt', required: true, includeIfNull: true)
  final int? revokedAt;

  @JsonKey(name: r'revokedBy', required: true, includeIfNull: true)
  final String? revokedBy;

  @JsonKey(name: r'revokeReason', required: true, includeIfNull: true)
  final String? revokeReason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AchievementGrant &&
          other.accountId == accountId &&
          other.achievementId == achievementId &&
          other.slug == slug &&
          other.name == name &&
          other.icon == icon &&
          other.definitionStatus == definitionStatus &&
          other.status == status &&
          other.awardReason == awardReason &&
          other.awardedAt == awardedAt &&
          other.awardedBy == awardedBy &&
          other.revokedAt == revokedAt &&
          other.revokedBy == revokedBy &&
          other.revokeReason == revokeReason;

  @override
  int get hashCode =>
      accountId.hashCode +
      achievementId.hashCode +
      slug.hashCode +
      name.hashCode +
      icon.hashCode +
      definitionStatus.hashCode +
      status.hashCode +
      (awardReason == null ? 0 : awardReason.hashCode) +
      awardedAt.hashCode +
      awardedBy.hashCode +
      (revokedAt == null ? 0 : revokedAt.hashCode) +
      (revokedBy == null ? 0 : revokedBy.hashCode) +
      (revokeReason == null ? 0 : revokeReason.hashCode);

  factory AchievementGrant.fromJson(Map<String, dynamic> json) =>
      _$AchievementGrantFromJson(json);

  Map<String, dynamic> toJson() => _$AchievementGrantToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AchievementGrantStatusEnum {
  @JsonValue(r'active')
  active(r'active'),
  @JsonValue(r'revoked')
  revoked(r'revoked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AchievementGrantStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
