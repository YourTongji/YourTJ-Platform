// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'subscription_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SubscriptionInput _$SubscriptionInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SubscriptionInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['targetType', 'targetId', 'level']);
      final val = SubscriptionInput(
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$SubscriptionInputTargetTypeEnumEnumMap,
            v,
            unknownValue: SubscriptionInputTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        targetId: $checkedConvert('targetId', (v) => v as String),
        level: $checkedConvert(
          'level',
          (v) => $enumDecode(
            _$SubscriptionInputLevelEnumEnumMap,
            v,
            unknownValue: SubscriptionInputLevelEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$SubscriptionInputToJson(
  SubscriptionInput instance,
) => <String, dynamic>{
  'targetType': _$SubscriptionInputTargetTypeEnumEnumMap[instance.targetType]!,
  'targetId': instance.targetId,
  'level': _$SubscriptionInputLevelEnumEnumMap[instance.level]!,
};

const _$SubscriptionInputTargetTypeEnumEnumMap = {
  SubscriptionInputTargetTypeEnum.board: 'board',
  SubscriptionInputTargetTypeEnum.thread: 'thread',
  SubscriptionInputTargetTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$SubscriptionInputLevelEnumEnumMap = {
  SubscriptionInputLevelEnum.watching: 'watching',
  SubscriptionInputLevelEnum.tracking: 'tracking',
  SubscriptionInputLevelEnum.muted: 'muted',
  SubscriptionInputLevelEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
