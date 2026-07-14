// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'reconciliation_run_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReconciliationRunInput _$ReconciliationRunInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ReconciliationRunInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = ReconciliationRunInput(
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$ReconciliationRunInputToJson(
  ReconciliationRunInput instance,
) => <String, dynamic>{'reason': instance.reason};
