// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_export_download_grant.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DataExportDownloadGrant _$DataExportDownloadGrantFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('DataExportDownloadGrant', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['token', 'expiresAt']);
  final val = DataExportDownloadGrant(
    token: $checkedConvert('token', (v) => v as String),
    expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$DataExportDownloadGrantToJson(
  DataExportDownloadGrant instance,
) => <String, dynamic>{
  'token': instance.token,
  'expiresAt': instance.expiresAt,
};
