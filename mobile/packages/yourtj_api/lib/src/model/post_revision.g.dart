// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'post_revision.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PostRevision _$PostRevisionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PostRevision', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'seq',
          'editorId',
          'oldTitle',
          'oldBody',
          'oldContentFormat',
          'oldContentVersion',
          'attachments',
          'createdAt',
        ],
      );
      final val = PostRevision(
        id: $checkedConvert('id', (v) => v as String),
        seq: $checkedConvert('seq', (v) => (v as num).toInt()),
        editorId: $checkedConvert('editorId', (v) => v as String),
        oldTitle: $checkedConvert('oldTitle', (v) => v as String?),
        oldBody: $checkedConvert('oldBody', (v) => v as String),
        oldContentFormat: $checkedConvert(
          'oldContentFormat',
          (v) => $enumDecode(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        oldContentVersion: $checkedConvert(
          'oldContentVersion',
          (v) => (v as num).toInt(),
        ),
        attachments: $checkedConvert(
          'attachments',
          (v) => (v as List<dynamic>)
              .map((e) => ForumAttachment.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$PostRevisionToJson(PostRevision instance) =>
    <String, dynamic>{
      'id': instance.id,
      'seq': instance.seq,
      'editorId': instance.editorId,
      'oldTitle': instance.oldTitle,
      'oldBody': instance.oldBody,
      'oldContentFormat': _$ContentFormatEnumMap[instance.oldContentFormat]!,
      'oldContentVersion': instance.oldContentVersion,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'createdAt': instance.createdAt,
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
