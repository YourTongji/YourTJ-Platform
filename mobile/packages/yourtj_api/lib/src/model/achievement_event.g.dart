// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_event.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementEvent _$AchievementEventFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AchievementEvent', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'achievementId',
          'slug',
          'name',
          'action',
          'source',
          'actorId',
          'reason',
          'createdAt',
        ],
      );
      final val = AchievementEvent(
        id: $checkedConvert('id', (v) => v as String),
        achievementId: $checkedConvert('achievementId', (v) => v as String),
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        action: $checkedConvert(
          'action',
          (v) => $enumDecode(
            _$AchievementEventActionEnumEnumMap,
            v,
            unknownValue: AchievementEventActionEnum.unknownDefaultOpenApi,
          ),
        ),
        source_: $checkedConvert(
          'source',
          (v) => $enumDecode(
            _$AchievementEventSource_EnumEnumMap,
            v,
            unknownValue: AchievementEventSource_Enum.unknownDefaultOpenApi,
          ),
        ),
        actorId: $checkedConvert('actorId', (v) => v as String?),
        reason: $checkedConvert('reason', (v) => v as String),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    }, fieldKeyMap: const {'source_': 'source'});

Map<String, dynamic> _$AchievementEventToJson(AchievementEvent instance) =>
    <String, dynamic>{
      'id': instance.id,
      'achievementId': instance.achievementId,
      'slug': instance.slug,
      'name': instance.name,
      'action': _$AchievementEventActionEnumEnumMap[instance.action]!,
      'source': _$AchievementEventSource_EnumEnumMap[instance.source_]!,
      'actorId': instance.actorId,
      'reason': instance.reason,
      'createdAt': instance.createdAt,
    };

const _$AchievementEventActionEnumEnumMap = {
  AchievementEventActionEnum.awarded: 'awarded',
  AchievementEventActionEnum.revoked: 'revoked',
  AchievementEventActionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AchievementEventSource_EnumEnumMap = {
  AchievementEventSource_Enum.automatic: 'automatic',
  AchievementEventSource_Enum.manual: 'manual',
  AchievementEventSource_Enum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
