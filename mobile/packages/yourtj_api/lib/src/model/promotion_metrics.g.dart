// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_metrics.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionMetrics _$PromotionMetricsFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PromotionMetrics', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['summary', 'days']);
      final val = PromotionMetrics(
        summary: $checkedConvert(
          'summary',
          (v) => PromotionMetricSummary.fromJson(v as Map<String, dynamic>),
        ),
        days: $checkedConvert(
          'days',
          (v) => (v as List<dynamic>)
              .map(
                (e) => PromotionMetricDay.fromJson(e as Map<String, dynamic>),
              )
              .toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$PromotionMetricsToJson(PromotionMetrics instance) =>
    <String, dynamic>{
      'summary': instance.summary.toJson(),
      'days': instance.days.map((e) => e.toJson()).toList(),
    };
