//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'product_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ProductInput {
  /// Returns a new [ProductInput] instance.
  ProductInput({
    required this.title,

    this.description,

    required this.price,

    required this.stock,

    this.deliveryInfo,
  });

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  // minimum: 1
  @JsonKey(name: r'price', required: true, includeIfNull: false)
  final int price;

  // minimum: 0
  @JsonKey(name: r'stock', required: true, includeIfNull: false)
  final int stock;

  @JsonKey(name: r'deliveryInfo', required: false, includeIfNull: false)
  final String? deliveryInfo;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ProductInput &&
          other.title == title &&
          other.description == description &&
          other.price == price &&
          other.stock == stock &&
          other.deliveryInfo == deliveryInfo;

  @override
  int get hashCode =>
      title.hashCode +
      description.hashCode +
      price.hashCode +
      stock.hashCode +
      deliveryInfo.hashCode;

  factory ProductInput.fromJson(Map<String, dynamic> json) =>
      _$ProductInputFromJson(json);

  Map<String, dynamic> toJson() => _$ProductInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
