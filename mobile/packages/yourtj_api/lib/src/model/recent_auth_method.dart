//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum RecentAuthMethod {
  @JsonValue(r'password')
  password(r'password'),
  @JsonValue(r'email_code')
  emailCode(r'email_code'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const RecentAuthMethod(this.value);

  final String value;

  @override
  String toString() => value;
}
