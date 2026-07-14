// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_reason_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminReasonInput _$AdminReasonInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminReasonInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['reason']);
      final val = AdminReasonInput(
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$AdminReasonInputToJson(AdminReasonInput instance) =>
    <String, dynamic>{'reason': instance.reason};
