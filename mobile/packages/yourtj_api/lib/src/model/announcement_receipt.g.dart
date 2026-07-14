// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_receipt.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementReceipt _$AnnouncementReceiptFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementReceipt', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['revision']);
  final val = AnnouncementReceipt(
    revision: $checkedConvert('revision', (v) => (v as num).toInt()),
    firstSeenAt: $checkedConvert('firstSeenAt', (v) => (v as num?)?.toInt()),
    dismissedAt: $checkedConvert('dismissedAt', (v) => (v as num?)?.toInt()),
    acknowledgedAt: $checkedConvert(
      'acknowledgedAt',
      (v) => (v as num?)?.toInt(),
    ),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementReceiptToJson(
  AnnouncementReceipt instance,
) => <String, dynamic>{
  'revision': instance.revision,
  'firstSeenAt': ?instance.firstSeenAt,
  'dismissedAt': ?instance.dismissedAt,
  'acknowledgedAt': ?instance.acknowledgedAt,
};
