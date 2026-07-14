// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'review_report_resolution_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReviewReportResolutionInput _$ReviewReportResolutionInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ReviewReportResolutionInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['action', 'note']);
  final val = ReviewReportResolutionInput(
    action: $checkedConvert(
      'action',
      (v) => $enumDecode(
        _$ReviewReportResolutionInputActionEnumEnumMap,
        v,
        unknownValue:
            ReviewReportResolutionInputActionEnum.unknownDefaultOpenApi,
      ),
    ),
    note: $checkedConvert('note', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$ReviewReportResolutionInputToJson(
  ReviewReportResolutionInput instance,
) => <String, dynamic>{
  'action': _$ReviewReportResolutionInputActionEnumEnumMap[instance.action]!,
  'note': instance.note,
};

const _$ReviewReportResolutionInputActionEnumEnumMap = {
  ReviewReportResolutionInputActionEnum.uphold: 'uphold',
  ReviewReportResolutionInputActionEnum.reject: 'reject',
  ReviewReportResolutionInputActionEnum.ignore: 'ignore',
  ReviewReportResolutionInputActionEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
