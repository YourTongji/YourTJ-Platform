// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'appeal_email_verification.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AppealEmailVerification _$AppealEmailVerificationFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AppealEmailVerification', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['email', 'code']);
  final val = AppealEmailVerification(
    email: $checkedConvert('email', (v) => v as String),
    code: $checkedConvert('code', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AppealEmailVerificationToJson(
  AppealEmailVerification instance,
) => <String, dynamic>{'email': instance.email, 'code': instance.code};
