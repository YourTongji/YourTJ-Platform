// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_report_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmReportInput _$DmReportInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmReportInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['reason']);
      final val = DmReportInput(
        reason: $checkedConvert(
          'reason',
          (v) => $enumDecode(
            _$DmReportInputReasonEnumEnumMap,
            v,
            unknownValue: DmReportInputReasonEnum.unknownDefaultOpenApi,
          ),
        ),
        note: $checkedConvert('note', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$DmReportInputToJson(DmReportInput instance) =>
    <String, dynamic>{
      'reason': _$DmReportInputReasonEnumEnumMap[instance.reason]!,
      'note': ?instance.note,
    };

const _$DmReportInputReasonEnumEnumMap = {
  DmReportInputReasonEnum.spam: 'spam',
  DmReportInputReasonEnum.abuse: 'abuse',
  DmReportInputReasonEnum.harassment: 'harassment',
  DmReportInputReasonEnum.fraud: 'fraud',
  DmReportInputReasonEnum.illegal: 'illegal',
  DmReportInputReasonEnum.other: 'other',
  DmReportInputReasonEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
