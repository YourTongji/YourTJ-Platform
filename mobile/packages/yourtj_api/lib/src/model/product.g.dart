// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'product.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Product _$ProductFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Product', json, ($checkedConvert) {
      final val = Product(
        id: $checkedConvert('id', (v) => v as String?),
        sellerId: $checkedConvert('sellerId', (v) => v as String?),
        title: $checkedConvert('title', (v) => v as String?),
        description: $checkedConvert('description', (v) => v as String?),
        price: $checkedConvert('price', (v) => (v as num?)?.toInt()),
        stock: $checkedConvert('stock', (v) => (v as num?)?.toInt()),
        status: $checkedConvert(
          'status',
          (v) => $enumDecodeNullable(
            _$ProductStatusEnumEnumMap,
            v,
            unknownValue: ProductStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ProductToJson(Product instance) => <String, dynamic>{
  'id': ?instance.id,
  'sellerId': ?instance.sellerId,
  'title': ?instance.title,
  'description': ?instance.description,
  'price': ?instance.price,
  'stock': ?instance.stock,
  'status': ?_$ProductStatusEnumEnumMap[instance.status],
  'createdAt': ?instance.createdAt,
};

const _$ProductStatusEnumEnumMap = {
  ProductStatusEnum.onSale: 'on_sale',
  ProductStatusEnum.offSale: 'off_sale',
  ProductStatusEnum.soldOut: 'sold_out',
  ProductStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
