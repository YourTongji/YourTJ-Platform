// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'subscription.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Subscription _$SubscriptionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Subscription', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['targetType', 'targetId', 'level', 'createdAt'],
      );
      final val = Subscription(
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$SubscriptionTargetTypeEnumEnumMap,
            v,
            unknownValue: SubscriptionTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        targetId: $checkedConvert('targetId', (v) => v as String),
        level: $checkedConvert(
          'level',
          (v) => $enumDecode(
            _$SubscriptionLevelEnumEnumMap,
            v,
            unknownValue: SubscriptionLevelEnum.unknownDefaultOpenApi,
          ),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$SubscriptionToJson(Subscription instance) =>
    <String, dynamic>{
      'targetType': _$SubscriptionTargetTypeEnumEnumMap[instance.targetType]!,
      'targetId': instance.targetId,
      'level': _$SubscriptionLevelEnumEnumMap[instance.level]!,
      'createdAt': instance.createdAt,
    };

const _$SubscriptionTargetTypeEnumEnumMap = {
  SubscriptionTargetTypeEnum.board: 'board',
  SubscriptionTargetTypeEnum.thread: 'thread',
  SubscriptionTargetTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$SubscriptionLevelEnumEnumMap = {
  SubscriptionLevelEnum.watching: 'watching',
  SubscriptionLevelEnum.tracking: 'tracking',
  SubscriptionLevelEnum.muted: 'muted',
  SubscriptionLevelEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
