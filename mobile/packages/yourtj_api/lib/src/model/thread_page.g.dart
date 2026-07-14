// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadPage _$ThreadPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = ThreadPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Thread.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$ThreadPageToJson(ThreadPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
