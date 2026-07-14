//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum DataExportStatus {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'running')
  running(r'running'),
  @JsonValue(r'ready')
  ready(r'ready'),
  @JsonValue(r'failed')
  failed(r'failed'),
  @JsonValue(r'expired')
  expired(r'expired'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DataExportStatus(this.value);

  final String value;

  @override
  String toString() => value;
}
