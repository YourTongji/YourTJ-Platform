// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'email_code_verification.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

EmailCodeVerification _$EmailCodeVerificationFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('EmailCodeVerification', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['email', 'code']);
  final val = EmailCodeVerification(
    email: $checkedConvert('email', (v) => v as String),
    code: $checkedConvert('code', (v) => v as String),
    purpose: $checkedConvert(
      'purpose',
      (v) => $enumDecodeNullable(
        _$EmailCodePurposeEnumMap,
        v,
        unknownValue: EmailCodePurpose.unknownDefaultOpenApi,
      ),
    ),
    handle: $checkedConvert('handle', (v) => v as String?),
    password: $checkedConvert('password', (v) => v as String?),
    clientInstallationId: $checkedConvert(
      'clientInstallationId',
      (v) => v as String?,
    ),
  );
  return val;
});

Map<String, dynamic> _$EmailCodeVerificationToJson(
  EmailCodeVerification instance,
) => <String, dynamic>{
  'email': instance.email,
  'code': instance.code,
  'purpose': ?_$EmailCodePurposeEnumMap[instance.purpose],
  'handle': ?instance.handle,
  'password': ?instance.password,
  'clientInstallationId': ?instance.clientInstallationId,
};

const _$EmailCodePurposeEnumMap = {
  EmailCodePurpose.login: 'login',
  EmailCodePurpose.registration: 'registration',
  EmailCodePurpose.appeal: 'appeal',
  EmailCodePurpose.recovery: 'recovery',
  EmailCodePurpose.unknownDefaultOpenApi: 'unknown_default_open_api',
};
