// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_reconciliation_report.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaReconciliationReport _$MediaReconciliationReportFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaReconciliationReport', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['dryRun', 'items', 'nextCursor', 'providerInventory'],
  );
  final val = MediaReconciliationReport(
    dryRun: $checkedConvert('dryRun', (v) => v as bool),
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map(
            (e) =>
                MediaReconciliationFinding.fromJson(e as Map<String, dynamic>),
          )
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    providerInventory: $checkedConvert(
      'providerInventory',
      (v) => MediaProviderInventoryStatus.fromJson(v as Map<String, dynamic>),
    ),
  );
  return val;
});

Map<String, dynamic> _$MediaReconciliationReportToJson(
  MediaReconciliationReport instance,
) => <String, dynamic>{
  'dryRun': instance.dryRun,
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'providerInventory': instance.providerInventory.toJson(),
};
