// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_thread_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserThreadPage _$UserThreadPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserThreadPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = UserThreadPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => UserThread.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$UserThreadPageToJson(UserThreadPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
