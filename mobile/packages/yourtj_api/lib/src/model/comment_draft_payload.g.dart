// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'comment_draft_payload.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CommentDraftPayload _$CommentDraftPayloadFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CommentDraftPayload', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'kind',
          'threadId',
          'body',
          'contentFormat',
          'parentId',
          'attachmentAssetIds',
        ],
      );
      final val = CommentDraftPayload(
        kind: $checkedConvert(
          'kind',
          (v) => $enumDecode(
            _$CommentDraftPayloadKindEnumEnumMap,
            v,
            unknownValue: CommentDraftPayloadKindEnum.unknownDefaultOpenApi,
          ),
        ),
        threadId: $checkedConvert('threadId', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecode(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        parentId: $checkedConvert('parentId', (v) => v as String?),
        attachmentAssetIds: $checkedConvert(
          'attachmentAssetIds',
          (v) => (v as List<dynamic>).map((e) => e as String).toSet(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$CommentDraftPayloadToJson(
  CommentDraftPayload instance,
) => <String, dynamic>{
  'kind': _$CommentDraftPayloadKindEnumEnumMap[instance.kind]!,
  'threadId': instance.threadId,
  'body': instance.body,
  'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
  'parentId': instance.parentId,
  'attachmentAssetIds': instance.attachmentAssetIds.toList(),
};

const _$CommentDraftPayloadKindEnumEnumMap = {
  CommentDraftPayloadKindEnum.comment: 'comment',
  CommentDraftPayloadKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
