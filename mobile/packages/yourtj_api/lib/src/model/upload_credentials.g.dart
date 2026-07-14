// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'upload_credentials.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UploadCredentials _$UploadCredentialsFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UploadCredentials', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'uploadIntentId',
          'accessKeyId',
          'accessKeySecret',
          'securityToken',
          'region',
          'bucket',
          'prefix',
          'ossKey',
          'callbackUrl',
          'callbackBody',
          'expiration',
        ],
      );
      final val = UploadCredentials(
        uploadIntentId: $checkedConvert('uploadIntentId', (v) => v as String),
        accessKeyId: $checkedConvert('accessKeyId', (v) => v as String),
        accessKeySecret: $checkedConvert('accessKeySecret', (v) => v as String),
        securityToken: $checkedConvert('securityToken', (v) => v as String),
        region: $checkedConvert('region', (v) => v as String),
        bucket: $checkedConvert('bucket', (v) => v as String),
        prefix: $checkedConvert('prefix', (v) => v as String),
        ossKey: $checkedConvert('ossKey', (v) => v as String),
        callbackUrl: $checkedConvert('callbackUrl', (v) => v as String),
        callbackBody: $checkedConvert('callbackBody', (v) => v as String),
        expiration: $checkedConvert('expiration', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$UploadCredentialsToJson(UploadCredentials instance) =>
    <String, dynamic>{
      'uploadIntentId': instance.uploadIntentId,
      'accessKeyId': instance.accessKeyId,
      'accessKeySecret': instance.accessKeySecret,
      'securityToken': instance.securityToken,
      'region': instance.region,
      'bucket': instance.bucket,
      'prefix': instance.prefix,
      'ossKey': instance.ossKey,
      'callbackUrl': instance.callbackUrl,
      'callbackBody': instance.callbackBody,
      'expiration': instance.expiration,
    };
