// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_draft_payload.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadDraftPayload _$ThreadDraftPayloadFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadDraftPayload', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'kind',
          'boardId',
          'title',
          'body',
          'contentFormat',
          'tags',
          'pollQuestion',
          'pollOptions',
          'attachmentAssetIds',
        ],
      );
      final val = ThreadDraftPayload(
        kind: $checkedConvert(
          'kind',
          (v) => $enumDecode(
            _$ThreadDraftPayloadKindEnumEnumMap,
            v,
            unknownValue: ThreadDraftPayloadKindEnum.unknownDefaultOpenApi,
          ),
        ),
        boardId: $checkedConvert('boardId', (v) => v as String?),
        title: $checkedConvert('title', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecode(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        tags: $checkedConvert(
          'tags',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
        pollQuestion: $checkedConvert('pollQuestion', (v) => v as String),
        pollOptions: $checkedConvert(
          'pollOptions',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
        attachmentAssetIds: $checkedConvert(
          'attachmentAssetIds',
          (v) => (v as List<dynamic>).map((e) => e as String).toSet(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$ThreadDraftPayloadToJson(ThreadDraftPayload instance) =>
    <String, dynamic>{
      'kind': _$ThreadDraftPayloadKindEnumEnumMap[instance.kind]!,
      'boardId': instance.boardId,
      'title': instance.title,
      'body': instance.body,
      'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
      'tags': instance.tags,
      'pollQuestion': instance.pollQuestion,
      'pollOptions': instance.pollOptions,
      'attachmentAssetIds': instance.attachmentAssetIds.toList(),
    };

const _$ThreadDraftPayloadKindEnumEnumMap = {
  ThreadDraftPayloadKindEnum.thread: 'thread',
  ThreadDraftPayloadKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};
