// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Achievement _$AchievementFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Achievement', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'slug',
          'name',
          'description',
          'icon',
          'status',
          'mintAmount',
          'version',
          'createdAt',
          'updatedAt',
        ],
      );
      final val = Achievement(
        id: $checkedConvert('id', (v) => v as String),
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        description: $checkedConvert('description', (v) => v as String?),
        icon: $checkedConvert(
          'icon',
          (v) => $enumDecode(
            _$AchievementIconEnumMap,
            v,
            unknownValue: AchievementIcon.unknownDefaultOpenApi,
          ),
        ),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$AchievementStatusEnumMap,
            v,
            unknownValue: AchievementStatus.unknownDefaultOpenApi,
          ),
        ),
        mintAmount: $checkedConvert('mintAmount', (v) => (v as num).toInt()),
        version: $checkedConvert('version', (v) => (v as num).toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AchievementToJson(Achievement instance) =>
    <String, dynamic>{
      'id': instance.id,
      'slug': instance.slug,
      'name': instance.name,
      'description': instance.description,
      'icon': _$AchievementIconEnumMap[instance.icon]!,
      'status': _$AchievementStatusEnumMap[instance.status]!,
      'mintAmount': instance.mintAmount,
      'version': instance.version,
      'createdAt': instance.createdAt,
      'updatedAt': instance.updatedAt,
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
