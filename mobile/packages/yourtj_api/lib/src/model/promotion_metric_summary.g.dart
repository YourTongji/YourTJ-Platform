// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_metric_summary.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionMetricSummary _$PromotionMetricSummaryFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('PromotionMetricSummary', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['from', 'to', 'impressions', 'clicks']);
  final val = PromotionMetricSummary(
    from: $checkedConvert('from', (v) => DateTime.parse(v as String)),
    to: $checkedConvert('to', (v) => DateTime.parse(v as String)),
    impressions: $checkedConvert('impressions', (v) => (v as num).toInt()),
    clicks: $checkedConvert('clicks', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$PromotionMetricSummaryToJson(
  PromotionMetricSummary instance,
) => <String, dynamic>{
  'from': instance.from.toIso8601String(),
  'to': instance.to.toIso8601String(),
  'impressions': instance.impressions,
  'clicks': instance.clicks,
};
