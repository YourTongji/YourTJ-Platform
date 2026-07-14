// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_message_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmMessagePage _$DmMessagePageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmMessagePage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = DmMessagePage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => DmMessage.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$DmMessagePageToJson(DmMessagePage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
