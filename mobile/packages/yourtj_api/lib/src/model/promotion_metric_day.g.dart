// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_metric_day.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionMetricDay _$PromotionMetricDayFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PromotionMetricDay', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['metricDate', 'impressions', 'clicks'],
      );
      final val = PromotionMetricDay(
        metricDate: $checkedConvert(
          'metricDate',
          (v) => DateTime.parse(v as String),
        ),
        impressions: $checkedConvert('impressions', (v) => (v as num).toInt()),
        clicks: $checkedConvert('clicks', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$PromotionMetricDayToJson(PromotionMetricDay instance) =>
    <String, dynamic>{
      'metricDate': instance.metricDate.toIso8601String(),
      'impressions': instance.impressions,
      'clicks': instance.clicks,
    };
