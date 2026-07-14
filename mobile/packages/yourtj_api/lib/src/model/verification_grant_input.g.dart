// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verification_grant_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VerificationGrantInput _$VerificationGrantInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('VerificationGrantInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['verificationTypeId', 'displayOnProfile', 'reason'],
  );
  final val = VerificationGrantInput(
    verificationTypeId: $checkedConvert(
      'verificationTypeId',
      (v) => v as String,
    ),
    displayOnProfile: $checkedConvert(
      'displayOnProfile',
      (v) => v as bool? ?? false,
    ),
    expiresAt: $checkedConvert('expiresAt', (v) => (v as num?)?.toInt()),
    evidenceReference: $checkedConvert(
      'evidenceReference',
      (v) => v as String?,
    ),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$VerificationGrantInputToJson(
  VerificationGrantInput instance,
) => <String, dynamic>{
  'verificationTypeId': instance.verificationTypeId,
  'displayOnProfile': instance.displayOnProfile,
  'expiresAt': ?instance.expiresAt,
  'evidenceReference': ?instance.evidenceReference,
  'reason': instance.reason,
};
