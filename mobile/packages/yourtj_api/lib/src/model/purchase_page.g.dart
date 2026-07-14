// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'purchase_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PurchasePage _$PurchasePageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PurchasePage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = PurchasePage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Purchase.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$PurchasePageToJson(PurchasePage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
