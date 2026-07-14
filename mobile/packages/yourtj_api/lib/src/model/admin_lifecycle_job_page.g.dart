// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_lifecycle_job_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminLifecycleJobPage _$AdminLifecycleJobPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminLifecycleJobPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = AdminLifecycleJobPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => AdminLifecycleJob.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$AdminLifecycleJobPageToJson(
  AdminLifecycleJobPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
