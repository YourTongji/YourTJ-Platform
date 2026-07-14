// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ignore_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

IgnorePage _$IgnorePageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('IgnorePage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = IgnorePage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => IgnoreUser.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$IgnorePageToJson(IgnorePage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
