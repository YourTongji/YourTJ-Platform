// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_export_job.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DataExportJob _$DataExportJobFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DataExportJob', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'status',
          'createdAt',
          'updatedAt',
          'expiresAt',
          'errorCode',
        ],
      );
      final val = DataExportJob(
        id: $checkedConvert('id', (v) => v as String),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$DataExportStatusEnumMap,
            v,
            unknownValue: DataExportStatus.unknownDefaultOpenApi,
          ),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
        errorCode: $checkedConvert('errorCode', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$DataExportJobToJson(DataExportJob instance) =>
    <String, dynamic>{
      'id': instance.id,
      'status': _$DataExportStatusEnumMap[instance.status]!,
      'createdAt': instance.createdAt,
      'updatedAt': instance.updatedAt,
      'expiresAt': instance.expiresAt,
      'errorCode': instance.errorCode,
    };

const _$DataExportStatusEnumMap = {
  DataExportStatus.queued: 'queued',
  DataExportStatus.running: 'running',
  DataExportStatus.ready: 'ready',
  DataExportStatus.failed: 'failed',
  DataExportStatus.expired: 'expired',
  DataExportStatus.unknownDefaultOpenApi: 'unknown_default_open_api',
};
