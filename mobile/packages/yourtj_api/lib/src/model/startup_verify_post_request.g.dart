// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'startup_verify_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

StartupVerifyPostRequest _$StartupVerifyPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('StartupVerifyPostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['token']);
  final val = StartupVerifyPostRequest(
    token: $checkedConvert('token', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$StartupVerifyPostRequestToJson(
  StartupVerifyPostRequest instance,
) => <String, dynamic>{'token': instance.token};
