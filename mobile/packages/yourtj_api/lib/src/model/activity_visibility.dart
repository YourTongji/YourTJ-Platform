//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// Controls profile activity lists, not the visibility of canonical public forum content.
enum ActivityVisibility {
  /// Controls profile activity lists, not the visibility of canonical public forum content.
  @JsonValue(r'public')
  public(r'public'),

  /// Controls profile activity lists, not the visibility of canonical public forum content.
  @JsonValue(r'campus')
  campus(r'campus'),

  /// Controls profile activity lists, not the visibility of canonical public forum content.
  @JsonValue(r'only_me')
  onlyMe(r'only_me'),

  /// Controls profile activity lists, not the visibility of canonical public forum content.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ActivityVisibility(this.value);

  final String value;

  @override
  String toString() => value;
}
