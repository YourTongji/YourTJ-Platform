// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'appeal.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Appeal _$AppealFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('Appeal', json, ($checkedConvert) {
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
    ],
  );
  final val = Appeal(
    id: $checkedConvert('id', (v) => v as String),
    governanceEventId: $checkedConvert('governanceEventId', (v) => v as String),
    originalAction: $checkedConvert('originalAction', (v) => v as String),
    originalReason: $checkedConvert('originalReason', (v) => v as String?),
    targetKind: $checkedConvert(
      'targetKind',
      (v) => $enumDecode(
        _$AppealTargetKindEnumEnumMap,
        v,
        unknownValue: AppealTargetKindEnum.unknownDefaultOpenApi,
      ),
    ),
    targetId: $checkedConvert('targetId', (v) => v as String),
    dispositionKind: $checkedConvert(
      'dispositionKind',
      (v) => $enumDecode(
        _$AppealDispositionKindEnumEnumMap,
        v,
        unknownValue: AppealDispositionKindEnum.unknownDefaultOpenApi,
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
  );
  return val;
});

Map<String, dynamic> _$AppealToJson(Appeal instance) => <String, dynamic>{
  'id': instance.id,
  'governanceEventId': instance.governanceEventId,
  'originalAction': instance.originalAction,
  'originalReason': ?instance.originalReason,
  'targetKind': _$AppealTargetKindEnumEnumMap[instance.targetKind]!,
  'targetId': instance.targetId,
  'dispositionKind':
      _$AppealDispositionKindEnumEnumMap[instance.dispositionKind]!,
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
};

const _$AppealTargetKindEnumEnumMap = {
  AppealTargetKindEnum.sanction: 'sanction',
  AppealTargetKindEnum.forumThread: 'forum_thread',
  AppealTargetKindEnum.forumComment: 'forum_comment',
  AppealTargetKindEnum.review: 'review',
  AppealTargetKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AppealDispositionKindEnumEnumMap = {
  AppealDispositionKindEnum.silence: 'silence',
  AppealDispositionKindEnum.suspend: 'suspend',
  AppealDispositionKindEnum.hide_: 'hide',
  AppealDispositionKindEnum.delete: 'delete',
  AppealDispositionKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
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
