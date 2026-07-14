// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'comment_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CommentInput _$CommentInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CommentInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['body']);
      final val = CommentInput(
        parentId: $checkedConvert('parentId', (v) => v as String?),
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
        quotedCommentId: $checkedConvert(
          'quotedCommentId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$CommentInputToJson(CommentInput instance) =>
    <String, dynamic>{
      'parentId': ?instance.parentId,
      'body': instance.body,
      'contentFormat': ?_$ContentFormatEnumMap[instance.contentFormat],
      'attachmentAssetIds': ?instance.attachmentAssetIds?.toList(),
      'quotedCommentId': ?instance.quotedCommentId,
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
