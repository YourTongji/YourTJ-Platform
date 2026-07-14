//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// Server-owned immutable variant profile. Clients cannot request arbitrary dimensions or object paths.
enum MediaDeliveryVariant {
  /// Server-owned immutable variant profile. Clients cannot request arbitrary dimensions or object paths.
  @JsonValue(r'thumb_256')
  thumb256(r'thumb_256'),

  /// Server-owned immutable variant profile. Clients cannot request arbitrary dimensions or object paths.
  @JsonValue(r'display_1280')
  display1280(r'display_1280'),

  /// Server-owned immutable variant profile. Clients cannot request arbitrary dimensions or object paths.
  @JsonValue(r'full_2048')
  full2048(r'full_2048'),

  /// Server-owned immutable variant profile. Clients cannot request arbitrary dimensions or object paths.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaDeliveryVariant(this.value);

  final String value;

  @override
  String toString() => value;
}
