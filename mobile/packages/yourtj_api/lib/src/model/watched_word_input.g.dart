// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'watched_word_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WatchedWordInput _$WatchedWordInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('WatchedWordInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['word', 'action', 'reason']);
      final val = WatchedWordInput(
        word: $checkedConvert('word', (v) => v as String),
        action: $checkedConvert(
          'action',
          (v) => $enumDecode(
            _$WatchedWordInputActionEnumEnumMap,
            v,
            unknownValue: WatchedWordInputActionEnum.unknownDefaultOpenApi,
          ),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$WatchedWordInputToJson(WatchedWordInput instance) =>
    <String, dynamic>{
      'word': instance.word,
      'action': _$WatchedWordInputActionEnumEnumMap[instance.action]!,
      'reason': instance.reason,
    };

const _$WatchedWordInputActionEnumEnumMap = {
  WatchedWordInputActionEnum.block: 'block',
  WatchedWordInputActionEnum.censor: 'censor',
  WatchedWordInputActionEnum.queue: 'queue',
  WatchedWordInputActionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
