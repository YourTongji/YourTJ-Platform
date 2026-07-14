// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'email_code_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

EmailCodeRequest _$EmailCodeRequestFromJson(Map<String, dynamic> json) =>
    $checkedCreate('EmailCodeRequest', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['email', 'captchaToken']);
      final val = EmailCodeRequest(
        email: $checkedConvert('email', (v) => v as String),
        captchaToken: $checkedConvert('captchaToken', (v) => v as String),
        purpose: $checkedConvert(
          'purpose',
          (v) => $enumDecodeNullable(
            _$EmailCodePurposeEnumMap,
            v,
            unknownValue: EmailCodePurpose.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$EmailCodeRequestToJson(EmailCodeRequest instance) =>
    <String, dynamic>{
      'email': instance.email,
      'captchaToken': instance.captchaToken,
      'purpose': ?_$EmailCodePurposeEnumMap[instance.purpose],
    };

const _$EmailCodePurposeEnumMap = {
  EmailCodePurpose.login: 'login',
  EmailCodePurpose.registration: 'registration',
  EmailCodePurpose.appeal: 'appeal',
  EmailCodePurpose.recovery: 'recovery',
  EmailCodePurpose.unknownDefaultOpenApi: 'unknown_default_open_api',
};
