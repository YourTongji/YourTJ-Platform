// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_receipt_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementReceiptInput _$AnnouncementReceiptInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementReceiptInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['revision', 'action']);
  final val = AnnouncementReceiptInput(
    revision: $checkedConvert('revision', (v) => (v as num).toInt()),
    action: $checkedConvert(
      'action',
      (v) => $enumDecode(
        _$AnnouncementReceiptInputActionEnumEnumMap,
        v,
        unknownValue: AnnouncementReceiptInputActionEnum.unknownDefaultOpenApi,
      ),
    ),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementReceiptInputToJson(
  AnnouncementReceiptInput instance,
) => <String, dynamic>{
  'revision': instance.revision,
  'action': _$AnnouncementReceiptInputActionEnumEnumMap[instance.action]!,
};

const _$AnnouncementReceiptInputActionEnumEnumMap = {
  AnnouncementReceiptInputActionEnum.seen: 'seen',
  AnnouncementReceiptInputActionEnum.dismiss: 'dismiss',
  AnnouncementReceiptInputActionEnum.acknowledge: 'acknowledge',
  AnnouncementReceiptInputActionEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
