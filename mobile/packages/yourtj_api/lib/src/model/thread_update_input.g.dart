// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadUpdateInput _$ThreadUpdateInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadUpdateInput', json, ($checkedConvert) {
      final val = ThreadUpdateInput(
        expectedVersion: $checkedConvert(
          'expectedVersion',
          (v) => (v as num?)?.toInt() ?? 1,
        ),
        title: $checkedConvert('title', (v) => v as String?),
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
      );
      return val;
    });

Map<String, dynamic> _$ThreadUpdateInputToJson(ThreadUpdateInput instance) =>
    <String, dynamic>{
      'expectedVersion': ?instance.expectedVersion,
      'title': ?instance.title,
      'body': ?instance.body,
      'contentFormat': ?_$ContentFormatEnumMap[instance.contentFormat],
      'tags': ?instance.tags?.toList(),
      'attachmentAssetIds': ?instance.attachmentAssetIds?.toList(),
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
