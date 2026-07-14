// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_comment.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserComment _$UserCommentFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserComment', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'threadId',
          'threadTitle',
          'body',
          'contentFormat',
          'replyCount',
          'voteCount',
          'viewerVote',
          'isBookmarked',
          'attachments',
          'createdAt',
        ],
      );
      final val = UserComment(
        id: $checkedConvert('id', (v) => v as String),
        threadId: $checkedConvert('threadId', (v) => v as String),
        threadTitle: $checkedConvert('threadTitle', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecode(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        replyCount: $checkedConvert('replyCount', (v) => (v as num).toInt()),
        voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
        viewerVote: $checkedConvert(
          'viewerVote',
          (v) => $enumDecodeNullable(
            _$UserCommentViewerVoteEnumEnumMap,
            v,
            unknownValue: UserCommentViewerVoteEnum.unknownDefaultOpenApi,
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

Map<String, dynamic> _$UserCommentToJson(UserComment instance) =>
    <String, dynamic>{
      'id': instance.id,
      'threadId': instance.threadId,
      'threadTitle': instance.threadTitle,
      'body': instance.body,
      'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
      'replyCount': instance.replyCount,
      'voteCount': instance.voteCount,
      'viewerVote': _$UserCommentViewerVoteEnumEnumMap[instance.viewerVote],
      'isBookmarked': instance.isBookmarked,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'createdAt': instance.createdAt,
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$UserCommentViewerVoteEnumEnumMap = {
  UserCommentViewerVoteEnum.up: 'up',
  UserCommentViewerVoteEnum.down: 'down',
  UserCommentViewerVoteEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
