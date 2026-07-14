//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
enum MediaDeliveryState {
  /// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
  @JsonValue(r'unpublished')
  unpublished(r'unpublished'),

  /// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
  @JsonValue(r'processing')
  processing(r'processing'),

  /// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
  @JsonValue(r'published')
  published(r'published'),

  /// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
  @JsonValue(r'failed')
  failed(r'failed'),

  /// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
  @JsonValue(r'blocked')
  blocked(r'blocked'),

  /// Sanitized Delivery projection state; moderation approval and CDN publication are separate facts.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaDeliveryState(this.value);

  final String value;

  @override
  String toString() => value;
}
