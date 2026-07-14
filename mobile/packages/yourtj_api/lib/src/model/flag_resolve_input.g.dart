// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'flag_resolve_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

FlagResolveInput _$FlagResolveInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('FlagResolveInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['action', 'note']);
      final val = FlagResolveInput(
        action: $checkedConvert(
          'action',
          (v) => $enumDecode(
            _$FlagResolveInputActionEnumEnumMap,
            v,
            unknownValue: FlagResolveInputActionEnum.unknownDefaultOpenApi,
          ),
        ),
        note: $checkedConvert('note', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$FlagResolveInputToJson(FlagResolveInput instance) =>
    <String, dynamic>{
      'action': _$FlagResolveInputActionEnumEnumMap[instance.action]!,
      'note': instance.note,
    };

const _$FlagResolveInputActionEnumEnumMap = {
  FlagResolveInputActionEnum.uphold: 'uphold',
  FlagResolveInputActionEnum.reject: 'reject',
  FlagResolveInputActionEnum.ignore: 'ignore',
  FlagResolveInputActionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
