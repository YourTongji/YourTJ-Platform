// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Page _$PageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Page', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = Page(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>).map((e) => e as Object).toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$PageToJson(Page instance) => <String, dynamic>{
  'items': instance.items,
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
