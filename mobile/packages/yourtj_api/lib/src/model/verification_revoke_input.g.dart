// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verification_revoke_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VerificationRevokeInput _$VerificationRevokeInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('VerificationRevokeInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = VerificationRevokeInput(
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$VerificationRevokeInputToJson(
  VerificationRevokeInput instance,
) => <String, dynamic>{'reason': instance.reason};
