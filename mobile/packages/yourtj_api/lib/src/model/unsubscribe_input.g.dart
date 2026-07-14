// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'unsubscribe_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UnsubscribeInput _$UnsubscribeInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UnsubscribeInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['targetType', 'targetId']);
      final val = UnsubscribeInput(
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$UnsubscribeInputTargetTypeEnumEnumMap,
            v,
            unknownValue: UnsubscribeInputTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        targetId: $checkedConvert('targetId', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$UnsubscribeInputToJson(
  UnsubscribeInput instance,
) => <String, dynamic>{
  'targetType': _$UnsubscribeInputTargetTypeEnumEnumMap[instance.targetType]!,
  'targetId': instance.targetId,
};

const _$UnsubscribeInputTargetTypeEnumEnumMap = {
  UnsubscribeInputTargetTypeEnum.board: 'board',
  UnsubscribeInputTargetTypeEnum.thread: 'thread',
  UnsubscribeInputTargetTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
