// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_recovery_email_verify_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AuthRecoveryEmailVerifyPostRequest _$AuthRecoveryEmailVerifyPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AuthRecoveryEmailVerifyPostRequest', json, (
  $checkedConvert,
) {
  $checkKeys(json, requiredKeys: const ['email', 'code']);
  final val = AuthRecoveryEmailVerifyPostRequest(
    email: $checkedConvert('email', (v) => v as String),
    code: $checkedConvert('code', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AuthRecoveryEmailVerifyPostRequestToJson(
  AuthRecoveryEmailVerifyPostRequest instance,
) => <String, dynamic>{'email': instance.email, 'code': instance.code};
