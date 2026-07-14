//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum VerificationCategory {
  @JsonValue(r'identity')
  identity(r'identity'),
  @JsonValue(r'special')
  special(r'special'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const VerificationCategory(this.value);

  final String value;

  @override
  String toString() => value;
}
