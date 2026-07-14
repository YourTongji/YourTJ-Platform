// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'health_status.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

HealthStatus _$HealthStatusFromJson(Map<String, dynamic> json) =>
    $checkedCreate('HealthStatus', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['status', 'service', 'version']);
      final val = HealthStatus(
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$HealthStatusStatusEnumEnumMap,
            v,
            unknownValue: HealthStatusStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        service: $checkedConvert('service', (v) => v as String),
        version: $checkedConvert('version', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$HealthStatusToJson(HealthStatus instance) =>
    <String, dynamic>{
      'status': _$HealthStatusStatusEnumEnumMap[instance.status]!,
      'service': instance.service,
      'version': instance.version,
    };

const _$HealthStatusStatusEnumEnumMap = {
  HealthStatusStatusEnum.ok: 'ok',
  HealthStatusStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
