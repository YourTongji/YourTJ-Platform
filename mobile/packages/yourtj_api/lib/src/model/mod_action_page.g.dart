// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mod_action_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ModActionPage _$ModActionPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ModActionPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = ModActionPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => ModAction.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$ModActionPageToJson(ModActionPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
