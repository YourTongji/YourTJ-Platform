// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'review_search_hit.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReviewSearchHit _$ReviewSearchHitFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ReviewSearchHit', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'courseId',
          'courseName',
          'rating',
          'comment',
          'approveCount',
          'createdAt',
        ],
      );
      final val = ReviewSearchHit(
        id: $checkedConvert('id', (v) => v as String),
        courseId: $checkedConvert('courseId', (v) => v as String),
        courseName: $checkedConvert('courseName', (v) => v as String),
        rating: $checkedConvert('rating', (v) => (v as num).toInt()),
        comment: $checkedConvert('comment', (v) => v as String?),
        approveCount: $checkedConvert(
          'approveCount',
          (v) => (v as num).toInt(),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ReviewSearchHitToJson(ReviewSearchHit instance) =>
    <String, dynamic>{
      'id': instance.id,
      'courseId': instance.courseId,
      'courseName': instance.courseName,
      'rating': instance.rating,
      'comment': instance.comment,
      'approveCount': instance.approveCount,
      'createdAt': instance.createdAt,
    };
