// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_password_login_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AuthPasswordLoginPostRequest _$AuthPasswordLoginPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AuthPasswordLoginPostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['email', 'password']);
  final val = AuthPasswordLoginPostRequest(
    email: $checkedConvert('email', (v) => v as String),
    password: $checkedConvert('password', (v) => v as String),
    clientInstallationId: $checkedConvert(
      'clientInstallationId',
      (v) => v as String?,
    ),
  );
  return val;
});

Map<String, dynamic> _$AuthPasswordLoginPostRequestToJson(
  AuthPasswordLoginPostRequest instance,
) => <String, dynamic>{
  'email': instance.email,
  'password': instance.password,
  'clientInstallationId': ?instance.clientInstallationId,
};
