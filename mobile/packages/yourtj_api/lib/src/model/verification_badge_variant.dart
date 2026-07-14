//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// Controlled semantic badge variant; arbitrary CSS and colors are not accepted.
enum VerificationBadgeVariant {
  /// Controlled semantic badge variant; arbitrary CSS and colors are not accepted.
  @JsonValue(r'default')
  default_(r'default'),

  /// Controlled semantic badge variant; arbitrary CSS and colors are not accepted.
  @JsonValue(r'secondary')
  secondary(r'secondary'),

  /// Controlled semantic badge variant; arbitrary CSS and colors are not accepted.
  @JsonValue(r'outline')
  outline(r'outline'),

  /// Controlled semantic badge variant; arbitrary CSS and colors are not accepted.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const VerificationBadgeVariant(this.value);

  final String value;

  @override
  String toString() => value;
}
