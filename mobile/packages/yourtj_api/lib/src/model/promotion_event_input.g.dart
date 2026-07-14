// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_event_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionEventInput _$PromotionEventInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PromotionEventInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['eventType', 'trackingToken']);
      final val = PromotionEventInput(
        eventType: $checkedConvert(
          'eventType',
          (v) => $enumDecode(
            _$PromotionEventInputEventTypeEnumEnumMap,
            v,
            unknownValue:
                PromotionEventInputEventTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        trackingToken: $checkedConvert('trackingToken', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$PromotionEventInputToJson(
  PromotionEventInput instance,
) => <String, dynamic>{
  'eventType': _$PromotionEventInputEventTypeEnumEnumMap[instance.eventType]!,
  'trackingToken': instance.trackingToken,
};

const _$PromotionEventInputEventTypeEnumEnumMap = {
  PromotionEventInputEventTypeEnum.impression: 'impression',
  PromotionEventInputEventTypeEnum.click: 'click',
  PromotionEventInputEventTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
