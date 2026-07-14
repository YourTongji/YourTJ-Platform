// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'comment_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CommentUpdateInput _$CommentUpdateInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CommentUpdateInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['body']);
      final val = CommentUpdateInput(
        expectedVersion: $checkedConvert(
          'expectedVersion',
          (v) => (v as num?)?.toInt() ?? 1,
        ),
        body: $checkedConvert('body', (v) => v as String),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecodeNullable(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        attachmentAssetIds: $checkedConvert(
          'attachmentAssetIds',
          (v) => (v as List<dynamic>?)?.map((e) => e as String).toSet(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$CommentUpdateInputToJson(CommentUpdateInput instance) =>
    <String, dynamic>{
      'expectedVersion': ?instance.expectedVersion,
      'body': instance.body,
      'contentFormat': ?_$ContentFormatEnumMap[instance.contentFormat],
      'attachmentAssetIds': ?instance.attachmentAssetIds?.toList(),
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
