// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_appeal_password_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AuthAppealPasswordPostRequest _$AuthAppealPasswordPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AuthAppealPasswordPostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['email', 'password']);
  final val = AuthAppealPasswordPostRequest(
    email: $checkedConvert('email', (v) => v as String),
    password: $checkedConvert('password', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AuthAppealPasswordPostRequestToJson(
  AuthAppealPasswordPostRequest instance,
) => <String, dynamic>{'email': instance.email, 'password': instance.password};
