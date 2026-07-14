// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionPage _$PromotionPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PromotionPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = PromotionPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Promotion.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$PromotionPageToJson(PromotionPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
