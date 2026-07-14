//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'product.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Product {
  /// Returns a new [Product] instance.
  Product({
    this.id,

    this.sellerId,

    this.title,

    this.description,

    this.price,

    this.stock,

    this.status,

    this.createdAt,
  });

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'sellerId', required: false, includeIfNull: false)
  final String? sellerId;

  @JsonKey(name: r'title', required: false, includeIfNull: false)
  final String? title;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  @JsonKey(name: r'price', required: false, includeIfNull: false)
  final int? price;

  @JsonKey(name: r'stock', required: false, includeIfNull: false)
  final int? stock;

  @JsonKey(
    name: r'status',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ProductStatusEnum.unknownDefaultOpenApi,
  )
  final ProductStatusEnum? status;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Product &&
          other.id == id &&
          other.sellerId == sellerId &&
          other.title == title &&
          other.description == description &&
          other.price == price &&
          other.stock == stock &&
          other.status == status &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      sellerId.hashCode +
      title.hashCode +
      (description == null ? 0 : description.hashCode) +
      price.hashCode +
      stock.hashCode +
      status.hashCode +
      createdAt.hashCode;

  factory Product.fromJson(Map<String, dynamic> json) =>
      _$ProductFromJson(json);

  Map<String, dynamic> toJson() => _$ProductToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ProductStatusEnum {
  @JsonValue(r'on_sale')
  onSale(r'on_sale'),
  @JsonValue(r'off_sale')
  offSale(r'off_sale'),
  @JsonValue(r'sold_out')
  soldOut(r'sold_out'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ProductStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
