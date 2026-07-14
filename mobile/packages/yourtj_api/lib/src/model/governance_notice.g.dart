// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'governance_notice.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

GovernanceNotice _$GovernanceNoticeFromJson(Map<String, dynamic> json) =>
    $checkedCreate('GovernanceNotice', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'noticeType',
          'subjectKind',
          'subjectId',
          'summary',
          'targetUrl',
          'read',
          'createdAt',
        ],
      );
      final val = GovernanceNotice(
        id: $checkedConvert('id', (v) => v as String),
        noticeType: $checkedConvert(
          'noticeType',
          (v) => $enumDecode(
            _$GovernanceNoticeNoticeTypeEnumEnumMap,
            v,
            unknownValue: GovernanceNoticeNoticeTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        subjectKind: $checkedConvert(
          'subjectKind',
          (v) => $enumDecode(
            _$GovernanceNoticeSubjectKindEnumEnumMap,
            v,
            unknownValue: GovernanceNoticeSubjectKindEnum.unknownDefaultOpenApi,
          ),
        ),
        subjectId: $checkedConvert('subjectId', (v) => v as String),
        summary: $checkedConvert('summary', (v) => v as String),
        appealId: $checkedConvert('appealId', (v) => v as String?),
        targetUrl: $checkedConvert('targetUrl', (v) => v as String),
        read: $checkedConvert('read', (v) => v as bool),
        readAt: $checkedConvert('readAt', (v) => (v as num?)?.toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$GovernanceNoticeToJson(
  GovernanceNotice instance,
) => <String, dynamic>{
  'id': instance.id,
  'noticeType': _$GovernanceNoticeNoticeTypeEnumEnumMap[instance.noticeType]!,
  'subjectKind':
      _$GovernanceNoticeSubjectKindEnumEnumMap[instance.subjectKind]!,
  'subjectId': instance.subjectId,
  'summary': instance.summary,
  'appealId': ?instance.appealId,
  'targetUrl': instance.targetUrl,
  'read': instance.read,
  'readAt': ?instance.readAt,
  'createdAt': instance.createdAt,
};

const _$GovernanceNoticeNoticeTypeEnumEnumMap = {
  GovernanceNoticeNoticeTypeEnum.sanctionApplied: 'sanction_applied',
  GovernanceNoticeNoticeTypeEnum.contentRestricted: 'content_restricted',
  GovernanceNoticeNoticeTypeEnum.appealSubmitted: 'appeal_submitted',
  GovernanceNoticeNoticeTypeEnum.appealInReview: 'appeal_in_review',
  GovernanceNoticeNoticeTypeEnum.appealUpheld: 'appeal_upheld',
  GovernanceNoticeNoticeTypeEnum.appealOverturned: 'appeal_overturned',
  GovernanceNoticeNoticeTypeEnum.appealAmended: 'appeal_amended',
  GovernanceNoticeNoticeTypeEnum.appealWithdrawn: 'appeal_withdrawn',
  GovernanceNoticeNoticeTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$GovernanceNoticeSubjectKindEnumEnumMap = {
  GovernanceNoticeSubjectKindEnum.sanction: 'sanction',
  GovernanceNoticeSubjectKindEnum.forumThread: 'forum_thread',
  GovernanceNoticeSubjectKindEnum.forumComment: 'forum_comment',
  GovernanceNoticeSubjectKindEnum.review: 'review',
  GovernanceNoticeSubjectKindEnum.appeal: 'appeal',
  GovernanceNoticeSubjectKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
