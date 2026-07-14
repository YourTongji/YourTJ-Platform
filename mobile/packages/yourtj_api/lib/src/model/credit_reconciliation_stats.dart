//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/credit_reconciliation_run.dart';
import 'package:json_annotation/json_annotation.dart';

part 'credit_reconciliation_stats.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CreditReconciliationStats {
  /// Returns a new [CreditReconciliationStats] instance.
  CreditReconciliationStats({
    required this.totalRuns,

    required this.failedRuns,

    required this.ledgerFailureRuns,

    required this.runsWithDrift,

    this.latestRun,
  });

  // minimum: 0
  @JsonKey(name: r'totalRuns', required: true, includeIfNull: false)
  final int totalRuns;

  // minimum: 0
  @JsonKey(name: r'failedRuns', required: true, includeIfNull: false)
  final int failedRuns;

  // minimum: 0
  @JsonKey(name: r'ledgerFailureRuns', required: true, includeIfNull: false)
  final int ledgerFailureRuns;

  // minimum: 0
  @JsonKey(name: r'runsWithDrift', required: true, includeIfNull: false)
  final int runsWithDrift;

  @JsonKey(name: r'latestRun', required: false, includeIfNull: false)
  final CreditReconciliationRun? latestRun;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CreditReconciliationStats &&
          other.totalRuns == totalRuns &&
          other.failedRuns == failedRuns &&
          other.ledgerFailureRuns == ledgerFailureRuns &&
          other.runsWithDrift == runsWithDrift &&
          other.latestRun == latestRun;

  @override
  int get hashCode =>
      totalRuns.hashCode +
      failedRuns.hashCode +
      ledgerFailureRuns.hashCode +
      runsWithDrift.hashCode +
      (latestRun == null ? 0 : latestRun.hashCode);

  factory CreditReconciliationStats.fromJson(Map<String, dynamic> json) =>
      _$CreditReconciliationStatsFromJson(json);

  Map<String, dynamic> toJson() => _$CreditReconciliationStatsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
