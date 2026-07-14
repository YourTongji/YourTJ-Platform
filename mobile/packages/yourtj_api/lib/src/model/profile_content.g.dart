// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_content.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfileContent _$ProfileContentFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ProfileContent', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'targetType',
          'id',
          'threadId',
          'title',
          'body',
          'contentFormat',
          'boardSlug',
          'authorHandle',
          'authorDisplayName',
          'replyCount',
          'voteCount',
          'viewerVote',
          'isBookmarked',
          'attachments',
          'createdAt',
          'activityAt',
        ],
      );
      final val = ProfileContent(
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$ProfileContentTargetTypeEnumEnumMap,
            v,
            unknownValue: ProfileContentTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        id: $checkedConvert('id', (v) => v as String),
        threadId: $checkedConvert('threadId', (v) => v as String),
        title: $checkedConvert('title', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String?),
        contentFormat: $checkedConvert(
          'contentFormat',
          (v) => $enumDecode(
            _$ContentFormatEnumMap,
            v,
            unknownValue: ContentFormat.unknownDefaultOpenApi,
          ),
        ),
        boardSlug: $checkedConvert('boardSlug', (v) => v as String),
        authorHandle: $checkedConvert('authorHandle', (v) => v as String),
        authorDisplayName: $checkedConvert(
          'authorDisplayName',
          (v) => v as String?,
        ),
        replyCount: $checkedConvert('replyCount', (v) => (v as num).toInt()),
        voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
        viewerVote: $checkedConvert(
          'viewerVote',
          (v) => $enumDecodeNullable(
            _$ProfileContentViewerVoteEnumEnumMap,
            v,
            unknownValue: ProfileContentViewerVoteEnum.unknownDefaultOpenApi,
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
        activityAt: $checkedConvert('activityAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ProfileContentToJson(ProfileContent instance) =>
    <String, dynamic>{
      'targetType': _$ProfileContentTargetTypeEnumEnumMap[instance.targetType]!,
      'id': instance.id,
      'threadId': instance.threadId,
      'title': instance.title,
      'body': instance.body,
      'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
      'boardSlug': instance.boardSlug,
      'authorHandle': instance.authorHandle,
      'authorDisplayName': instance.authorDisplayName,
      'replyCount': instance.replyCount,
      'voteCount': instance.voteCount,
      'viewerVote': _$ProfileContentViewerVoteEnumEnumMap[instance.viewerVote],
      'isBookmarked': instance.isBookmarked,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'createdAt': instance.createdAt,
      'activityAt': instance.activityAt,
    };

const _$ProfileContentTargetTypeEnumEnumMap = {
  ProfileContentTargetTypeEnum.thread: 'thread',
  ProfileContentTargetTypeEnum.comment: 'comment',
  ProfileContentTargetTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ProfileContentViewerVoteEnumEnumMap = {
  ProfileContentViewerVoteEnum.up: 'up',
  ProfileContentViewerVoteEnum.down: 'down',
  ProfileContentViewerVoteEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
