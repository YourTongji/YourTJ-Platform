//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'promotion_metric_summary.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PromotionMetricSummary {
  /// Returns a new [PromotionMetricSummary] instance.
  PromotionMetricSummary({
    required this.from,

    required this.to,

    required this.impressions,

    required this.clicks,
  });

  @JsonKey(name: r'from', required: true, includeIfNull: false)
  final DateTime from;

  @JsonKey(name: r'to', required: true, includeIfNull: false)
  final DateTime to;

  // minimum: 0
  @JsonKey(name: r'impressions', required: true, includeIfNull: false)
  final int impressions;

  // minimum: 0
  @JsonKey(name: r'clicks', required: true, includeIfNull: false)
  final int clicks;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PromotionMetricSummary &&
          other.from == from &&
          other.to == to &&
          other.impressions == impressions &&
          other.clicks == clicks;

  @override
  int get hashCode =>
      from.hashCode + to.hashCode + impressions.hashCode + clicks.hashCode;

  factory PromotionMetricSummary.fromJson(Map<String, dynamic> json) =>
      _$PromotionMetricSummaryFromJson(json);

  Map<String, dynamic> toJson() => _$PromotionMetricSummaryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
