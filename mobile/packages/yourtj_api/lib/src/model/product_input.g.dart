// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'product_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProductInput _$ProductInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ProductInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['title', 'price', 'stock']);
      final val = ProductInput(
        title: $checkedConvert('title', (v) => v as String),
        description: $checkedConvert('description', (v) => v as String?),
        price: $checkedConvert('price', (v) => (v as num).toInt()),
        stock: $checkedConvert('stock', (v) => (v as num).toInt()),
        deliveryInfo: $checkedConvert('deliveryInfo', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$ProductInputToJson(ProductInput instance) =>
    <String, dynamic>{
      'title': instance.title,
      'description': ?instance.description,
      'price': instance.price,
      'stock': instance.stock,
      'deliveryInfo': ?instance.deliveryInfo,
    };
