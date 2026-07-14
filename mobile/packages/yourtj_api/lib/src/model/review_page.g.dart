// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'review_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReviewPage _$ReviewPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ReviewPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = ReviewPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Review.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$ReviewPageToJson(ReviewPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
