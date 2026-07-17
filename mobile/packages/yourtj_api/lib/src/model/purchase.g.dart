// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'purchase.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Purchase _$PurchaseFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Purchase', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'productId',
          'buyerId',
          'sellerId',
          'amount',
          'status',
          'deliveryInfo',
          'createdAt',
        ],
      );
      final val = Purchase(
        id: $checkedConvert('id', (v) => v as String),
        productId: $checkedConvert('productId', (v) => v as String),
        buyerId: $checkedConvert('buyerId', (v) => v as String),
        sellerId: $checkedConvert('sellerId', (v) => v as String),
        amount: $checkedConvert('amount', (v) => (v as num).toInt()),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$PurchaseStatusEnumEnumMap,
            v,
            unknownValue: PurchaseStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        deliveryInfo: $checkedConvert('deliveryInfo', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$PurchaseToJson(Purchase instance) => <String, dynamic>{
  'id': instance.id,
  'productId': instance.productId,
  'buyerId': instance.buyerId,
  'sellerId': instance.sellerId,
  'amount': instance.amount,
  'status': _$PurchaseStatusEnumEnumMap[instance.status]!,
  'deliveryInfo': instance.deliveryInfo,
  'createdAt': instance.createdAt,
};

const _$PurchaseStatusEnumEnumMap = {
  PurchaseStatusEnum.pending: 'pending',
  PurchaseStatusEnum.accepted: 'accepted',
  PurchaseStatusEnum.delivered: 'delivered',
  PurchaseStatusEnum.completed: 'completed',
  PurchaseStatusEnum.cancelled: 'cancelled',
  PurchaseStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
