// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'report.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Report _$ReportFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Report', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['id', 'reviewId', 'reason', 'status', 'createdAt'],
      );
      final val = Report(
        id: $checkedConvert('id', (v) => v as String),
        reviewId: $checkedConvert('reviewId', (v) => v as String),
        reason: $checkedConvert('reason', (v) => v as String),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$ReportStatusEnumEnumMap,
            v,
            unknownValue: ReportStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        courseId: $checkedConvert('courseId', (v) => v as String?),
        reviewAuthorHandle: $checkedConvert(
          'reviewAuthorHandle',
          (v) => v as String?,
        ),
        reviewRating: $checkedConvert(
          'reviewRating',
          (v) => (v as num?)?.toInt(),
        ),
        reviewStatus: $checkedConvert(
          'reviewStatus',
          (v) => $enumDecodeNullable(
            _$ReportReviewStatusEnumEnumMap,
            v,
            unknownValue: ReportReviewStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        reviewExcerpt: $checkedConvert('reviewExcerpt', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ReportToJson(Report instance) => <String, dynamic>{
  'id': instance.id,
  'reviewId': instance.reviewId,
  'reason': instance.reason,
  'status': _$ReportStatusEnumEnumMap[instance.status]!,
  'courseId': ?instance.courseId,
  'reviewAuthorHandle': ?instance.reviewAuthorHandle,
  'reviewRating': ?instance.reviewRating,
  'reviewStatus': ?_$ReportReviewStatusEnumEnumMap[instance.reviewStatus],
  'reviewExcerpt': ?instance.reviewExcerpt,
  'createdAt': instance.createdAt,
};

const _$ReportStatusEnumEnumMap = {
  ReportStatusEnum.open: 'open',
  ReportStatusEnum.upheld: 'upheld',
  ReportStatusEnum.rejected: 'rejected',
  ReportStatusEnum.ignored: 'ignored',
  ReportStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ReportReviewStatusEnumEnumMap = {
  ReportReviewStatusEnum.visible: 'visible',
  ReportReviewStatusEnum.hidden: 'hidden',
  ReportReviewStatusEnum.pending: 'pending',
  ReportReviewStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
