// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TaskPage _$TaskPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TaskPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = TaskPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Task.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$TaskPageToJson(TaskPage instance) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
