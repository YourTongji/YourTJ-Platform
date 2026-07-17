//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'purchase.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Purchase {
  /// Returns a new [Purchase] instance.
  Purchase({
    required this.id,

    required this.productId,

    required this.buyerId,

    required this.sellerId,

    required this.amount,

    required this.status,

    required this.deliveryInfo,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'productId', required: true, includeIfNull: false)
  final String productId;

  @JsonKey(name: r'buyerId', required: true, includeIfNull: false)
  final String buyerId;

  @JsonKey(name: r'sellerId', required: true, includeIfNull: false)
  final String sellerId;

  @JsonKey(name: r'amount', required: true, includeIfNull: false)
  final int amount;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PurchaseStatusEnum.unknownDefaultOpenApi,
  )
  final PurchaseStatusEnum status;

  /// Visible only to the purchase buyer and seller
  @JsonKey(name: r'deliveryInfo', required: true, includeIfNull: true)
  final String? deliveryInfo;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Purchase &&
          other.id == id &&
          other.productId == productId &&
          other.buyerId == buyerId &&
          other.sellerId == sellerId &&
          other.amount == amount &&
          other.status == status &&
          other.deliveryInfo == deliveryInfo &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      productId.hashCode +
      buyerId.hashCode +
      sellerId.hashCode +
      amount.hashCode +
      status.hashCode +
      (deliveryInfo == null ? 0 : deliveryInfo.hashCode) +
      createdAt.hashCode;

  factory Purchase.fromJson(Map<String, dynamic> json) =>
      _$PurchaseFromJson(json);

  Map<String, dynamic> toJson() => _$PurchaseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum PurchaseStatusEnum {
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'accepted')
  accepted(r'accepted'),
  @JsonValue(r'delivered')
  delivered(r'delivered'),
  @JsonValue(r'completed')
  completed(r'completed'),
  @JsonValue(r'cancelled')
  cancelled(r'cancelled'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PurchaseStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
