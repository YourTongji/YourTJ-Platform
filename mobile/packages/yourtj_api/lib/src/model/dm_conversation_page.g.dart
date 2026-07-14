// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_conversation_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmConversationPage _$DmConversationPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmConversationPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = DmConversationPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => DmConversation.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$DmConversationPageToJson(DmConversationPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
