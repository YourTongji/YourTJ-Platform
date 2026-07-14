// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_review_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CreateReviewInput _$CreateReviewInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CreateReviewInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['rating', 'captchaToken']);
      final val = CreateReviewInput(
        rating: $checkedConvert('rating', (v) => (v as num).toInt()),
        comment: $checkedConvert('comment', (v) => v as String?),
        semester: $checkedConvert('semester', (v) => v as String?),
        score: $checkedConvert('score', (v) => v as String?),
        captchaToken: $checkedConvert('captchaToken', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$CreateReviewInputToJson(CreateReviewInput instance) =>
    <String, dynamic>{
      'rating': instance.rating,
      'comment': ?instance.comment,
      'semester': ?instance.semester,
      'score': ?instance.score,
      'captchaToken': instance.captchaToken,
    };
