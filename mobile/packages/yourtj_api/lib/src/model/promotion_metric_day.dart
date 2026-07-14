//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'promotion_metric_day.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PromotionMetricDay {
  /// Returns a new [PromotionMetricDay] instance.
  PromotionMetricDay({
    required this.metricDate,

    required this.impressions,

    required this.clicks,
  });

  @JsonKey(name: r'metricDate', required: true, includeIfNull: false)
  final DateTime metricDate;

  // minimum: 0
  @JsonKey(name: r'impressions', required: true, includeIfNull: false)
  final int impressions;

  // minimum: 0
  @JsonKey(name: r'clicks', required: true, includeIfNull: false)
  final int clicks;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PromotionMetricDay &&
          other.metricDate == metricDate &&
          other.impressions == impressions &&
          other.clicks == clicks;

  @override
  int get hashCode =>
      metricDate.hashCode + impressions.hashCode + clicks.hashCode;

  factory PromotionMetricDay.fromJson(Map<String, dynamic> json) =>
      _$PromotionMetricDayFromJson(json);

  Map<String, dynamic> toJson() => _$PromotionMetricDayToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
