// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'watched_word.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WatchedWord _$WatchedWordFromJson(Map<String, dynamic> json) =>
    $checkedCreate('WatchedWord', json, ($checkedConvert) {
      final val = WatchedWord(
        id: $checkedConvert('id', (v) => v as String?),
        word: $checkedConvert('word', (v) => v as String?),
        action: $checkedConvert(
          'action',
          (v) => $enumDecodeNullable(
            _$WatchedWordActionEnumEnumMap,
            v,
            unknownValue: WatchedWordActionEnum.unknownDefaultOpenApi,
          ),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$WatchedWordToJson(WatchedWord instance) =>
    <String, dynamic>{
      'id': ?instance.id,
      'word': ?instance.word,
      'action': ?_$WatchedWordActionEnumEnumMap[instance.action],
      'createdAt': ?instance.createdAt,
    };

const _$WatchedWordActionEnumEnumMap = {
  WatchedWordActionEnum.block: 'block',
  WatchedWordActionEnum.censor: 'censor',
  WatchedWordActionEnum.queue: 'queue',
  WatchedWordActionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
