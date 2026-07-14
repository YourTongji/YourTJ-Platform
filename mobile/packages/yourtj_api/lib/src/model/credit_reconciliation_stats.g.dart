// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credit_reconciliation_stats.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CreditReconciliationStats _$CreditReconciliationStatsFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('CreditReconciliationStats', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'totalRuns',
      'failedRuns',
      'ledgerFailureRuns',
      'runsWithDrift',
    ],
  );
  final val = CreditReconciliationStats(
    totalRuns: $checkedConvert('totalRuns', (v) => (v as num).toInt()),
    failedRuns: $checkedConvert('failedRuns', (v) => (v as num).toInt()),
    ledgerFailureRuns: $checkedConvert(
      'ledgerFailureRuns',
      (v) => (v as num).toInt(),
    ),
    runsWithDrift: $checkedConvert('runsWithDrift', (v) => (v as num).toInt()),
    latestRun: $checkedConvert(
      'latestRun',
      (v) => v == null
          ? null
          : CreditReconciliationRun.fromJson(v as Map<String, dynamic>),
    ),
  );
  return val;
});

Map<String, dynamic> _$CreditReconciliationStatsToJson(
  CreditReconciliationStats instance,
) => <String, dynamic>{
  'totalRuns': instance.totalRuns,
  'failedRuns': instance.failedRuns,
  'ledgerFailureRuns': instance.ledgerFailureRuns,
  'runsWithDrift': instance.runsWithDrift,
  'latestRun': ?instance.latestRun?.toJson(),
};
