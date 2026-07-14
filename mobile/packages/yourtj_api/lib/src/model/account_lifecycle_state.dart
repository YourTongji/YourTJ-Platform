//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum AccountLifecycleState {
  @JsonValue(r'active')
  active(r'active'),
  @JsonValue(r'deactivated')
  deactivated(r'deactivated'),
  @JsonValue(r'deletion_requested')
  deletionRequested(r'deletion_requested'),
  @JsonValue(r'deleted')
  deleted(r'deleted'),
  @JsonValue(r'purged')
  purged(r'purged'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AccountLifecycleState(this.value);

  final String value;

  @override
  String toString() => value;
}
