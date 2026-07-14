// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vote_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VoteInput _$VoteInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('VoteInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['value', 'postType']);
      final val = VoteInput(
        value: $checkedConvert(
          'value',
          (v) => $enumDecode(
            _$VoteInputValueEnumEnumMap,
            v,
            unknownValue: VoteInputValueEnum.unknownDefaultOpenApi,
          ),
        ),
        postType: $checkedConvert(
          'postType',
          (v) => $enumDecode(
            _$VoteInputPostTypeEnumEnumMap,
            v,
            unknownValue: VoteInputPostTypeEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$VoteInputToJson(VoteInput instance) => <String, dynamic>{
  'value': _$VoteInputValueEnumEnumMap[instance.value]!,
  'postType': _$VoteInputPostTypeEnumEnumMap[instance.postType]!,
};

const _$VoteInputValueEnumEnumMap = {
  VoteInputValueEnum.up: 'up',
  VoteInputValueEnum.down: 'down',
  VoteInputValueEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$VoteInputPostTypeEnumEnumMap = {
  VoteInputPostTypeEnum.thread: 'thread',
  VoteInputPostTypeEnum.comment: 'comment',
  VoteInputPostTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
