//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum RelationshipListVisibility {
  @JsonValue(r'public')
  public(r'public'),
  @JsonValue(r'campus')
  campus(r'campus'),
  @JsonValue(r'followers')
  followers(r'followers'),
  @JsonValue(r'only_me')
  onlyMe(r'only_me'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const RelationshipListVisibility(this.value);

  final String value;

  @override
  String toString() => value;
}
