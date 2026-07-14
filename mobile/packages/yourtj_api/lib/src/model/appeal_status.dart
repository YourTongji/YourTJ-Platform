//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum AppealStatus {
  @JsonValue(r'submitted')
  submitted(r'submitted'),
  @JsonValue(r'in_review')
  inReview(r'in_review'),
  @JsonValue(r'upheld')
  upheld(r'upheld'),
  @JsonValue(r'overturned')
  overturned(r'overturned'),
  @JsonValue(r'amended')
  amended(r'amended'),
  @JsonValue(r'withdrawn')
  withdrawn(r'withdrawn'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AppealStatus(this.value);

  final String value;

  @override
  String toString() => value;
}
