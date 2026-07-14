// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'review_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReviewInput _$ReviewInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ReviewInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['rating']);
      final val = ReviewInput(
        rating: $checkedConvert('rating', (v) => (v as num).toInt()),
        comment: $checkedConvert('comment', (v) => v as String?),
        semester: $checkedConvert('semester', (v) => v as String?),
        score: $checkedConvert('score', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$ReviewInputToJson(ReviewInput instance) =>
    <String, dynamic>{
      'rating': instance.rating,
      'comment': ?instance.comment,
      'semester': ?instance.semester,
      'score': ?instance.score,
    };
