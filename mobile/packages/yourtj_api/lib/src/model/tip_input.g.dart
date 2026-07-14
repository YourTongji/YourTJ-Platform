// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tip_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TipInput _$TipInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TipInput', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['toAccountId', 'amount', 'targetType', 'targetId'],
      );
      final val = TipInput(
        toAccountId: $checkedConvert('toAccountId', (v) => v as String),
        amount: $checkedConvert('amount', (v) => (v as num).toInt()),
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$TipInputTargetTypeEnumEnumMap,
            v,
            unknownValue: TipInputTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        targetId: $checkedConvert('targetId', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$TipInputToJson(TipInput instance) => <String, dynamic>{
  'toAccountId': instance.toAccountId,
  'amount': instance.amount,
  'targetType': _$TipInputTargetTypeEnumEnumMap[instance.targetType]!,
  'targetId': instance.targetId,
};

const _$TipInputTargetTypeEnumEnumMap = {
  TipInputTargetTypeEnum.review: 'review',
  TipInputTargetTypeEnum.thread: 'thread',
  TipInputTargetTypeEnum.comment: 'comment',
  TipInputTargetTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
