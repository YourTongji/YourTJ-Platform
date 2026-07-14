//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'health_status.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class HealthStatus {
  /// Returns a new [HealthStatus] instance.
  HealthStatus({
    required this.status,

    required this.service,

    required this.version,
  });

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: HealthStatusStatusEnum.unknownDefaultOpenApi,
  )
  final HealthStatusStatusEnum status;

  @JsonKey(name: r'service', required: true, includeIfNull: false)
  final String service;

  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final String version;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is HealthStatus &&
          other.status == status &&
          other.service == service &&
          other.version == version;

  @override
  int get hashCode => status.hashCode + service.hashCode + version.hashCode;

  factory HealthStatus.fromJson(Map<String, dynamic> json) =>
      _$HealthStatusFromJson(json);

  Map<String, dynamic> toJson() => _$HealthStatusToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum HealthStatusStatusEnum {
  @JsonValue(r'ok')
  ok(r'ok'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const HealthStatusStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
