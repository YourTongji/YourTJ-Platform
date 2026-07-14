// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_grant.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementGrant _$AchievementGrantFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AchievementGrant', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'accountId',
          'achievementId',
          'slug',
          'name',
          'icon',
          'definitionStatus',
          'status',
          'awardReason',
          'awardedAt',
          'awardedBy',
          'revokedAt',
          'revokedBy',
          'revokeReason',
        ],
      );
      final val = AchievementGrant(
        accountId: $checkedConvert('accountId', (v) => v as String),
        achievementId: $checkedConvert('achievementId', (v) => v as String),
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        icon: $checkedConvert(
          'icon',
          (v) => $enumDecode(
            _$AchievementIconEnumMap,
            v,
            unknownValue: AchievementIcon.unknownDefaultOpenApi,
          ),
        ),
        definitionStatus: $checkedConvert(
          'definitionStatus',
          (v) => $enumDecode(
            _$AchievementStatusEnumMap,
            v,
            unknownValue: AchievementStatus.unknownDefaultOpenApi,
          ),
        ),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$AchievementGrantStatusEnumEnumMap,
            v,
            unknownValue: AchievementGrantStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        awardReason: $checkedConvert('awardReason', (v) => v as String?),
        awardedAt: $checkedConvert('awardedAt', (v) => (v as num).toInt()),
        awardedBy: $checkedConvert('awardedBy', (v) => v as String),
        revokedAt: $checkedConvert('revokedAt', (v) => (v as num?)?.toInt()),
        revokedBy: $checkedConvert('revokedBy', (v) => v as String?),
        revokeReason: $checkedConvert('revokeReason', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$AchievementGrantToJson(
  AchievementGrant instance,
) => <String, dynamic>{
  'accountId': instance.accountId,
  'achievementId': instance.achievementId,
  'slug': instance.slug,
  'name': instance.name,
  'icon': _$AchievementIconEnumMap[instance.icon]!,
  'definitionStatus': _$AchievementStatusEnumMap[instance.definitionStatus]!,
  'status': _$AchievementGrantStatusEnumEnumMap[instance.status]!,
  'awardReason': instance.awardReason,
  'awardedAt': instance.awardedAt,
  'awardedBy': instance.awardedBy,
  'revokedAt': instance.revokedAt,
  'revokedBy': instance.revokedBy,
  'revokeReason': instance.revokeReason,
};

const _$AchievementIconEnumMap = {
  AchievementIcon.award: 'award',
  AchievementIcon.bookOpenCheck: 'book-open-check',
  AchievementIcon.messageCircleHeart: 'message-circle-heart',
  AchievementIcon.star: 'star',
  AchievementIcon.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AchievementStatusEnumMap = {
  AchievementStatus.active: 'active',
  AchievementStatus.retired: 'retired',
  AchievementStatus.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AchievementGrantStatusEnumEnumMap = {
  AchievementGrantStatusEnum.active: 'active',
  AchievementGrantStatusEnum.revoked: 'revoked',
  AchievementGrantStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
