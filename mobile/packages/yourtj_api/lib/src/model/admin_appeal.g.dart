// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_appeal.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminAppeal _$AdminAppealFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminAppeal', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'governanceEventId',
      'originalAction',
      'targetKind',
      'targetId',
      'dispositionKind',
      'status',
      'submissionReason',
      'submittedAt',
      'appealableUntil',
      'version',
      'history',
      'appellantAccountId',
    ],
  );
  final val = AdminAppeal(
    id: $checkedConvert('id', (v) => v as String),
    governanceEventId: $checkedConvert('governanceEventId', (v) => v as String),
    originalAction: $checkedConvert('originalAction', (v) => v as String),
    originalReason: $checkedConvert('originalReason', (v) => v as String?),
    targetKind: $checkedConvert(
      'targetKind',
      (v) => $enumDecode(
        _$AdminAppealTargetKindEnumEnumMap,
        v,
        unknownValue: AdminAppealTargetKindEnum.unknownDefaultOpenApi,
      ),
    ),
    targetId: $checkedConvert('targetId', (v) => v as String),
    dispositionKind: $checkedConvert(
      'dispositionKind',
      (v) => $enumDecode(
        _$AdminAppealDispositionKindEnumEnumMap,
        v,
        unknownValue: AdminAppealDispositionKindEnum.unknownDefaultOpenApi,
      ),
    ),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$AppealStatusEnumMap,
        v,
        unknownValue: AppealStatus.unknownDefaultOpenApi,
      ),
    ),
    submissionReason: $checkedConvert('submissionReason', (v) => v as String),
    submittedAt: $checkedConvert('submittedAt', (v) => (v as num).toInt()),
    appealableUntil: $checkedConvert(
      'appealableUntil',
      (v) => (v as num).toInt(),
    ),
    reviewStartedAt: $checkedConvert(
      'reviewStartedAt',
      (v) => (v as num?)?.toInt(),
    ),
    decisionReason: $checkedConvert('decisionReason', (v) => v as String?),
    amendment: $checkedConvert('amendment', (v) => v),
    decidedAt: $checkedConvert('decidedAt', (v) => (v as num?)?.toInt()),
    version: $checkedConvert('version', (v) => (v as num).toInt()),
    history: $checkedConvert(
      'history',
      (v) => (v as List<dynamic>)
          .map((e) => AppealHistory.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    appellantAccountId: $checkedConvert(
      'appellantAccountId',
      (v) => v as String,
    ),
    reviewerAccountId: $checkedConvert(
      'reviewerAccountId',
      (v) => v as String?,
    ),
  );
  return val;
});

Map<String, dynamic> _$AdminAppealToJson(AdminAppeal instance) =>
    <String, dynamic>{
      'id': instance.id,
      'governanceEventId': instance.governanceEventId,
      'originalAction': instance.originalAction,
      'originalReason': ?instance.originalReason,
      'targetKind': _$AdminAppealTargetKindEnumEnumMap[instance.targetKind]!,
      'targetId': instance.targetId,
      'dispositionKind':
          _$AdminAppealDispositionKindEnumEnumMap[instance.dispositionKind]!,
      'status': _$AppealStatusEnumMap[instance.status]!,
      'submissionReason': instance.submissionReason,
      'submittedAt': instance.submittedAt,
      'appealableUntil': instance.appealableUntil,
      'reviewStartedAt': ?instance.reviewStartedAt,
      'decisionReason': ?instance.decisionReason,
      'amendment': ?instance.amendment,
      'decidedAt': ?instance.decidedAt,
      'version': instance.version,
      'history': instance.history.map((e) => e.toJson()).toList(),
      'appellantAccountId': instance.appellantAccountId,
      'reviewerAccountId': ?instance.reviewerAccountId,
    };

const _$AdminAppealTargetKindEnumEnumMap = {
  AdminAppealTargetKindEnum.sanction: 'sanction',
  AdminAppealTargetKindEnum.forumThread: 'forum_thread',
  AdminAppealTargetKindEnum.forumComment: 'forum_comment',
  AdminAppealTargetKindEnum.review: 'review',
  AdminAppealTargetKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AdminAppealDispositionKindEnumEnumMap = {
  AdminAppealDispositionKindEnum.silence: 'silence',
  AdminAppealDispositionKindEnum.suspend: 'suspend',
  AdminAppealDispositionKindEnum.hide_: 'hide',
  AdminAppealDispositionKindEnum.delete: 'delete',
  AdminAppealDispositionKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AppealStatusEnumMap = {
  AppealStatus.submitted: 'submitted',
  AppealStatus.inReview: 'in_review',
  AppealStatus.upheld: 'upheld',
  AppealStatus.overturned: 'overturned',
  AppealStatus.amended: 'amended',
  AppealStatus.withdrawn: 'withdrawn',
  AppealStatus.unknownDefaultOpenApi: 'unknown_default_open_api',
};
