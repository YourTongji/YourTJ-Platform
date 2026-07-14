// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'flag_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

FlagInput _$FlagInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('FlagInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['reason', 'postType']);
      final val = FlagInput(
        reason: $checkedConvert(
          'reason',
          (v) => $enumDecode(
            _$FlagInputReasonEnumEnumMap,
            v,
            unknownValue: FlagInputReasonEnum.unknownDefaultOpenApi,
          ),
        ),
        note: $checkedConvert('note', (v) => v as String?),
        postType: $checkedConvert(
          'postType',
          (v) => $enumDecode(
            _$FlagInputPostTypeEnumEnumMap,
            v,
            unknownValue: FlagInputPostTypeEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$FlagInputToJson(FlagInput instance) => <String, dynamic>{
  'reason': _$FlagInputReasonEnumEnumMap[instance.reason]!,
  'note': ?instance.note,
  'postType': _$FlagInputPostTypeEnumEnumMap[instance.postType]!,
};

const _$FlagInputReasonEnumEnumMap = {
  FlagInputReasonEnum.spam: 'spam',
  FlagInputReasonEnum.abuse: 'abuse',
  FlagInputReasonEnum.offTopic: 'off_topic',
  FlagInputReasonEnum.illegal: 'illegal',
  FlagInputReasonEnum.other: 'other',
  FlagInputReasonEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$FlagInputPostTypeEnumEnumMap = {
  FlagInputPostTypeEnum.thread: 'thread',
  FlagInputPostTypeEnum.comment: 'comment',
  FlagInputPostTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
