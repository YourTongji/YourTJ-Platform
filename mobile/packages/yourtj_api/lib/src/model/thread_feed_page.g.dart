// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_feed_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadFeedPage _$ThreadFeedPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadFeedPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = ThreadFeedPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => ThreadFeed.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$ThreadFeedPageToJson(ThreadFeedPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
