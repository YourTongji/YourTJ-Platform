//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum EmailCodePurpose {
  @JsonValue(r'login')
  login(r'login'),
  @JsonValue(r'registration')
  registration(r'registration'),
  @JsonValue(r'appeal')
  appeal(r'appeal'),
  @JsonValue(r'recovery')
  recovery(r'recovery'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const EmailCodePurpose(this.value);

  final String value;

  @override
  String toString() => value;
}
