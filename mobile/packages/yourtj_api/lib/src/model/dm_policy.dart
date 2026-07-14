//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// following means accounts the recipient follows may start a new conversation.
enum DmPolicy {
  /// following means accounts the recipient follows may start a new conversation.
  @JsonValue(r'everyone')
  everyone(r'everyone'),

  /// following means accounts the recipient follows may start a new conversation.
  @JsonValue(r'following')
  following(r'following'),

  /// following means accounts the recipient follows may start a new conversation.
  @JsonValue(r'nobody')
  nobody(r'nobody'),

  /// following means accounts the recipient follows may start a new conversation.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmPolicy(this.value);

  final String value;

  @override
  String toString() => value;
}
