// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_versioned_archive_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminVersionedArchiveInput _$AdminVersionedArchiveInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminVersionedArchiveInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['expectedVersion', 'reason']);
  final val = AdminVersionedArchiveInput(
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminVersionedArchiveInputToJson(
  AdminVersionedArchiveInput instance,
) => <String, dynamic>{
  'expectedVersion': instance.expectedVersion,
  'reason': instance.reason,
};
