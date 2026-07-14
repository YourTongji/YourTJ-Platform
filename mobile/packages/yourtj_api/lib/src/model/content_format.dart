//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum ContentFormat {
  @JsonValue(r'plain_v1')
  plainV1(r'plain_v1'),
  @JsonValue(r'markdown_v1')
  markdownV1(r'markdown_v1'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ContentFormat(this.value);

  final String value;

  @override
  String toString() => value;
}
