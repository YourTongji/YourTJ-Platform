// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_counts.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmCounts _$DmCountsFromJson(Map<String, dynamic> json) => $checkedCreate(
  'DmCounts',
  json,
  ($checkedConvert) {
    $checkKeys(
      json,
      requiredKeys: const ['count', 'unreadCount', 'requestCount'],
    );
    final val = DmCounts(
      count: $checkedConvert('count', (v) => (v as num).toInt()),
      unreadCount: $checkedConvert('unreadCount', (v) => (v as num).toInt()),
      requestCount: $checkedConvert('requestCount', (v) => (v as num).toInt()),
    );
    return val;
  },
);

Map<String, dynamic> _$DmCountsToJson(DmCounts instance) => <String, dynamic>{
  'count': instance.count,
  'unreadCount': instance.unreadCount,
  'requestCount': instance.requestCount,
};
