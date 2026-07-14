//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_delivery_variant.dart';
import 'package:json_annotation/json_annotation.dart';

part 'media_delivery.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaDelivery {
  /// Returns a new [MediaDelivery] instance.
  MediaDelivery({
    required this.assetId,

    required this.variant,

    required this.url,

    required this.expiresAt,

    required this.mime,

    required this.width,

    required this.height,
  });

  @JsonKey(name: r'assetId', required: true, includeIfNull: false)
  final String assetId;

  @JsonKey(
    name: r'variant',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeliveryVariant.unknownDefaultOpenApi,
  )
  final MediaDeliveryVariant variant;

  /// Five-minute CDN URL whose path is an opaque immutable Delivery identifier, not an Ingest key or provider URL.
  @JsonKey(name: r'url', required: true, includeIfNull: false)
  final String url;

  /// Unix seconds; clients should refresh before this time.
  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @JsonKey(
    name: r'mime',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeliveryMimeEnum.unknownDefaultOpenApi,
  )
  final MediaDeliveryMimeEnum mime;

  // minimum: 1
  // maximum: 2048
  @JsonKey(name: r'width', required: true, includeIfNull: false)
  final int width;

  // minimum: 1
  // maximum: 2048
  @JsonKey(name: r'height', required: true, includeIfNull: false)
  final int height;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaDelivery &&
          other.assetId == assetId &&
          other.variant == variant &&
          other.url == url &&
          other.expiresAt == expiresAt &&
          other.mime == mime &&
          other.width == width &&
          other.height == height;

  @override
  int get hashCode =>
      assetId.hashCode +
      variant.hashCode +
      url.hashCode +
      expiresAt.hashCode +
      mime.hashCode +
      width.hashCode +
      height.hashCode;

  factory MediaDelivery.fromJson(Map<String, dynamic> json) =>
      _$MediaDeliveryFromJson(json);

  Map<String, dynamic> toJson() => _$MediaDeliveryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum MediaDeliveryMimeEnum {
  @JsonValue(r'image/webp')
  imageSlashWebp(r'image/webp'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaDeliveryMimeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
