// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'selection_sync_job_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SelectionSyncJobPage _$SelectionSyncJobPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('SelectionSyncJobPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = SelectionSyncJobPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => SelectionSyncJob.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$SelectionSyncJobPageToJson(
  SelectionSyncJobPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
