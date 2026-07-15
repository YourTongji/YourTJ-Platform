// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'review.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Review _$ReviewFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('Review', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['viewerLiked', 'canEdit', 'canReport']);
  final val = Review(
    id: $checkedConvert('id', (v) => v as String?),
    courseId: $checkedConvert('courseId', (v) => v as String?),
    rating: $checkedConvert('rating', (v) => (v as num?)?.toInt()),
    comment: $checkedConvert('comment', (v) => v as String?),
    score: $checkedConvert('score', (v) => v as String?),
    semester: $checkedConvert('semester', (v) => v as String?),
    authorHandle: $checkedConvert('authorHandle', (v) => v as String?),
    authorAvatar: $checkedConvert('authorAvatar', (v) => v as String?),
    approveCount: $checkedConvert('approveCount', (v) => (v as num?)?.toInt()),
    viewerLiked: $checkedConvert('viewerLiked', (v) => v as bool),
    canEdit: $checkedConvert('canEdit', (v) => v as bool),
    canReport: $checkedConvert('canReport', (v) => v as bool),
    status: $checkedConvert(
      'status',
      (v) => $enumDecodeNullable(
        _$ReviewStatusEnumEnumMap,
        v,
        unknownValue: ReviewStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
  );
  return val;
});

Map<String, dynamic> _$ReviewToJson(Review instance) => <String, dynamic>{
  'id': ?instance.id,
  'courseId': ?instance.courseId,
  'rating': ?instance.rating,
  'comment': ?instance.comment,
  'score': ?instance.score,
  'semester': ?instance.semester,
  'authorHandle': ?instance.authorHandle,
  'authorAvatar': ?instance.authorAvatar,
  'approveCount': ?instance.approveCount,
  'viewerLiked': instance.viewerLiked,
  'canEdit': instance.canEdit,
  'canReport': instance.canReport,
  'status': ?_$ReviewStatusEnumEnumMap[instance.status],
  'createdAt': ?instance.createdAt,
};

const _$ReviewStatusEnumEnumMap = {
  ReviewStatusEnum.visible: 'visible',
  ReviewStatusEnum.hidden: 'hidden',
  ReviewStatusEnum.pending: 'pending',
  ReviewStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
