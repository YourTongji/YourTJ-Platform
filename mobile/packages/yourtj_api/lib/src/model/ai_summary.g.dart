// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ai_summary.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AiSummary _$AiSummaryFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AiSummary', json, ($checkedConvert) {
      final val = AiSummary(
        courseId: $checkedConvert('courseId', (v) => v as String?),
        summary: $checkedConvert('summary', (v) => v as String?),
        model: $checkedConvert('model', (v) => v as String?),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AiSummaryToJson(AiSummary instance) => <String, dynamic>{
  'courseId': ?instance.courseId,
  'summary': ?instance.summary,
  'model': ?instance.model,
  'updatedAt': ?instance.updatedAt,
};
