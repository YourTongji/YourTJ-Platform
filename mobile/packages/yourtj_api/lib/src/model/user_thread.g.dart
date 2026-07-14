// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_thread.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserThread _$UserThreadFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserThread', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'title',
          'bodyExcerpt',
          'contentFormat',
          'boardSlug',
          'replyCount',
          'voteCount',
          'viewerVote',
          'isBookmarked',
          'attachments',
          'createdAt',
        ],
      );
      final val = UserThread(
        id: $checkedConvert('id', (v) => v as String),
        title: $checkedConvert('title', (v) => v as String),
        bodyExcerpt: $checkedConvert('bodyExcerpt', (v) => v as String?),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecode(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        boardSlug: $checkedConvert('boardSlug', (v) => v as String),
        replyCount: $checkedConvert('replyCount', (v) => (v as num).toInt()),
        voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
        viewerVote: $checkedConvert(
          'viewerVote',
          (v) => $enumDecodeNullable(
            _$UserThreadViewerVoteEnumEnumMap,
            v,
            unknownValue: UserThreadViewerVoteEnum.unknownDefaultOpenApi,
          ),
        ),
        isBookmarked: $checkedConvert('isBookmarked', (v) => v as bool),
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

Map<String, dynamic> _$UserThreadToJson(UserThread instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'bodyExcerpt': instance.bodyExcerpt,
      'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
      'boardSlug': instance.boardSlug,
      'replyCount': instance.replyCount,
      'voteCount': instance.voteCount,
      'viewerVote': _$UserThreadViewerVoteEnumEnumMap[instance.viewerVote],
      'isBookmarked': instance.isBookmarked,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'createdAt': instance.createdAt,
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$UserThreadViewerVoteEnumEnumMap = {
  UserThreadViewerVoteEnum.up: 'up',
  UserThreadViewerVoteEnum.down: 'down',
  UserThreadViewerVoteEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
