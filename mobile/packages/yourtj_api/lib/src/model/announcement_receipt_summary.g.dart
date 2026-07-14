// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_receipt_summary.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementReceiptSummary _$AnnouncementReceiptSummaryFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementReceiptSummary', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['seenCount', 'dismissedCount', 'acknowledgedCount'],
  );
  final val = AnnouncementReceiptSummary(
    seenCount: $checkedConvert('seenCount', (v) => (v as num).toInt()),
    dismissedCount: $checkedConvert(
      'dismissedCount',
      (v) => (v as num).toInt(),
    ),
    acknowledgedCount: $checkedConvert(
      'acknowledgedCount',
      (v) => (v as num).toInt(),
    ),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementReceiptSummaryToJson(
  AnnouncementReceiptSummary instance,
) => <String, dynamic>{
  'seenCount': instance.seenCount,
  'dismissedCount': instance.dismissedCount,
  'acknowledgedCount': instance.acknowledgedCount,
};
