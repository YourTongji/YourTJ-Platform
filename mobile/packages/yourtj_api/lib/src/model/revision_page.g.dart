// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'revision_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RevisionPage _$RevisionPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('RevisionPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = RevisionPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => PostRevision.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$RevisionPageToJson(RevisionPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
