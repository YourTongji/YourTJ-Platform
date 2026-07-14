// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_forum_flag.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminForumFlag _$AdminForumFlagFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminForumFlag', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'targetType',
          'targetId',
          'reporterId',
          'reason',
          'weight',
          'status',
          'createdAt',
        ],
      );
      final val = AdminForumFlag(
        id: $checkedConvert('id', (v) => v as String),
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$AdminForumFlagTargetTypeEnumEnumMap,
            v,
            unknownValue: AdminForumFlagTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        targetId: $checkedConvert('targetId', (v) => v as String),
        reporterId: $checkedConvert('reporterId', (v) => v as String),
        reason: $checkedConvert(
          'reason',
          (v) => $enumDecode(
            _$AdminForumFlagReasonEnumEnumMap,
            v,
            unknownValue: AdminForumFlagReasonEnum.unknownDefaultOpenApi,
          ),
        ),
        note: $checkedConvert('note', (v) => v as String?),
        weight: $checkedConvert('weight', (v) => v as num),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$AdminForumFlagStatusEnumEnumMap,
            v,
            unknownValue: AdminForumFlagStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        handledBy: $checkedConvert('handledBy', (v) => v as String?),
        handledAt: $checkedConvert('handledAt', (v) => (v as num?)?.toInt()),
        resolutionNote: $checkedConvert('resolutionNote', (v) => v as String?),
        authorHandle: $checkedConvert('authorHandle', (v) => v as String?),
        targetTitle: $checkedConvert('targetTitle', (v) => v as String?),
        contentExcerpt: $checkedConvert('contentExcerpt', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AdminForumFlagToJson(AdminForumFlag instance) =>
    <String, dynamic>{
      'id': instance.id,
      'targetType': _$AdminForumFlagTargetTypeEnumEnumMap[instance.targetType]!,
      'targetId': instance.targetId,
      'reporterId': instance.reporterId,
      'reason': _$AdminForumFlagReasonEnumEnumMap[instance.reason]!,
      'note': ?instance.note,
      'weight': instance.weight,
      'status': _$AdminForumFlagStatusEnumEnumMap[instance.status]!,
      'handledBy': ?instance.handledBy,
      'handledAt': ?instance.handledAt,
      'resolutionNote': ?instance.resolutionNote,
      'authorHandle': ?instance.authorHandle,
      'targetTitle': ?instance.targetTitle,
      'contentExcerpt': ?instance.contentExcerpt,
      'createdAt': instance.createdAt,
    };

const _$AdminForumFlagTargetTypeEnumEnumMap = {
  AdminForumFlagTargetTypeEnum.thread: 'thread',
  AdminForumFlagTargetTypeEnum.comment: 'comment',
  AdminForumFlagTargetTypeEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AdminForumFlagReasonEnumEnumMap = {
  AdminForumFlagReasonEnum.spam: 'spam',
  AdminForumFlagReasonEnum.abuse: 'abuse',
  AdminForumFlagReasonEnum.offTopic: 'off_topic',
  AdminForumFlagReasonEnum.illegal: 'illegal',
  AdminForumFlagReasonEnum.other: 'other',
  AdminForumFlagReasonEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AdminForumFlagStatusEnumEnumMap = {
  AdminForumFlagStatusEnum.open: 'open',
  AdminForumFlagStatusEnum.upheld: 'upheld',
  AdminForumFlagStatusEnum.rejected: 'rejected',
  AdminForumFlagStatusEnum.ignored: 'ignored',
  AdminForumFlagStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
