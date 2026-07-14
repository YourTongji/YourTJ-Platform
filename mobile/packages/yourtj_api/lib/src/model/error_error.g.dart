// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'error_error.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ErrorError _$ErrorErrorFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ErrorError', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['code', 'message']);
      final val = ErrorError(
        code: $checkedConvert('code', (v) => v as String),
        message: $checkedConvert('message', (v) => v as String),
        details: $checkedConvert('details', (v) => v),
      );
      return val;
    });

Map<String, dynamic> _$ErrorErrorToJson(ErrorError instance) =>
    <String, dynamic>{
      'code': instance.code,
      'message': instance.message,
      'details': ?instance.details,
    };
