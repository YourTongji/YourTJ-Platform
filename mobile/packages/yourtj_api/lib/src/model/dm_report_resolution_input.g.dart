// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_report_resolution_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmReportResolutionInput _$DmReportResolutionInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('DmReportResolutionInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['action']);
  final val = DmReportResolutionInput(
    action: $checkedConvert(
      'action',
      (v) => $enumDecode(
        _$DmReportResolutionInputActionEnumEnumMap,
        v,
        unknownValue: DmReportResolutionInputActionEnum.unknownDefaultOpenApi,
      ),
    ),
    note: $checkedConvert('note', (v) => v as String?),
  );
  return val;
});

Map<String, dynamic> _$DmReportResolutionInputToJson(
  DmReportResolutionInput instance,
) => <String, dynamic>{
  'action': _$DmReportResolutionInputActionEnumEnumMap[instance.action]!,
  'note': ?instance.note,
};

const _$DmReportResolutionInputActionEnumEnumMap = {
  DmReportResolutionInputActionEnum.uphold: 'uphold',
  DmReportResolutionInputActionEnum.reject: 'reject',
  DmReportResolutionInputActionEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
