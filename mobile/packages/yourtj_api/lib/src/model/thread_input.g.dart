// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadInput _$ThreadInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['boardId', 'title']);
      final val = ThreadInput(
        boardId: $checkedConvert('boardId', (v) => v as String),
        title: $checkedConvert('title', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String?),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecodeNullable(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        tags: $checkedConvert(
          'tags',
          (v) => (v as List<dynamic>?)?.map((e) => e as String).toSet(),
        ),
        attachmentAssetIds: $checkedConvert(
          'attachmentAssetIds',
          (v) => (v as List<dynamic>?)?.map((e) => e as String).toSet(),
        ),
        poll: $checkedConvert(
          'poll',
          (v) =>
              v == null ? null : PollInput.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$ThreadInputToJson(ThreadInput instance) =>
    <String, dynamic>{
      'boardId': instance.boardId,
      'title': instance.title,
      'body': ?instance.body,
      'contentFormat': ?_$ContentFormatEnumMap[instance.contentFormat],
      'tags': ?instance.tags?.toList(),
      'attachmentAssetIds': ?instance.attachmentAssetIds?.toList(),
      'poll': ?instance.poll?.toJson(),
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
