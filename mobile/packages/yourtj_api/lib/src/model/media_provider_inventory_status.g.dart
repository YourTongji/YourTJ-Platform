// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_provider_inventory_status.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaProviderInventoryStatus _$MediaProviderInventoryStatusFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaProviderInventoryStatus', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'state',
      'ingestCandidateCount',
      'deliveryCandidateCount',
    ],
  );
  final val = MediaProviderInventoryStatus(
    state: $checkedConvert(
      'state',
      (v) => $enumDecode(
        _$MediaProviderInventoryStatusStateEnumEnumMap,
        v,
        unknownValue:
            MediaProviderInventoryStatusStateEnum.unknownDefaultOpenApi,
      ),
    ),
    ingestCandidateCount: $checkedConvert(
      'ingestCandidateCount',
      (v) => (v as num).toInt(),
    ),
    deliveryCandidateCount: $checkedConvert(
      'deliveryCandidateCount',
      (v) => (v as num).toInt(),
    ),
  );
  return val;
});

Map<String, dynamic> _$MediaProviderInventoryStatusToJson(
  MediaProviderInventoryStatus instance,
) => <String, dynamic>{
  'state': _$MediaProviderInventoryStatusStateEnumEnumMap[instance.state]!,
  'ingestCandidateCount': instance.ingestCandidateCount,
  'deliveryCandidateCount': instance.deliveryCandidateCount,
};

const _$MediaProviderInventoryStatusStateEnumEnumMap = {
  MediaProviderInventoryStatusStateEnum.manualInventoryRequired:
      'manual_inventory_required',
  MediaProviderInventoryStatusStateEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
