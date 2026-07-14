// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'appeal_access_token.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AppealAccessToken _$AppealAccessTokenFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AppealAccessToken', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['accessToken', 'expiresAt']);
      final val = AppealAccessToken(
        accessToken: $checkedConvert('accessToken', (v) => v as String),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AppealAccessTokenToJson(AppealAccessToken instance) =>
    <String, dynamic>{
      'accessToken': instance.accessToken,
      'expiresAt': instance.expiresAt,
    };
