//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/promotion_metric_summary.dart';
import 'package:yourtj_api/src/model/promotion_metric_day.dart';
import 'package:json_annotation/json_annotation.dart';

part 'promotion_metrics.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PromotionMetrics {
  /// Returns a new [PromotionMetrics] instance.
  PromotionMetrics({required this.summary, required this.days});

  @JsonKey(name: r'summary', required: true, includeIfNull: false)
  final PromotionMetricSummary summary;

  @JsonKey(name: r'days', required: true, includeIfNull: false)
  final List<PromotionMetricDay> days;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PromotionMetrics &&
          other.summary == summary &&
          other.days == days;

  @override
  int get hashCode => summary.hashCode + days.hashCode;

  factory PromotionMetrics.fromJson(Map<String, dynamic> json) =>
      _$PromotionMetricsFromJson(json);

  Map<String, dynamic> toJson() => _$PromotionMetricsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
