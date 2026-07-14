// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_password_forgot_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AuthPasswordForgotPostRequest _$AuthPasswordForgotPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AuthPasswordForgotPostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['email', 'captchaToken']);
  final val = AuthPasswordForgotPostRequest(
    email: $checkedConvert('email', (v) => v as String),
    captchaToken: $checkedConvert('captchaToken', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AuthPasswordForgotPostRequestToJson(
  AuthPasswordForgotPostRequest instance,
) => <String, dynamic>{
  'email': instance.email,
  'captchaToken': instance.captchaToken,
};
