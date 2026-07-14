//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/activity_weights.dart';
import 'package:json_annotation/json_annotation.dart';

part 'activity_policy.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ActivityPolicy {
  /// Returns a new [ActivityPolicy] instance.
  ActivityPolicy({
    required this.version,

    required this.timezone,

    required this.weights,

    required this.reason,

    required this.changedBy,

    required this.createdAt,
  });

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  @JsonKey(
    name: r'timezone',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ActivityPolicyTimezoneEnum.unknownDefaultOpenApi,
  )
  final ActivityPolicyTimezoneEnum timezone;

  @JsonKey(name: r'weights', required: true, includeIfNull: false)
  final ActivityWeights weights;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'changedBy', required: true, includeIfNull: false)
  final String changedBy;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ActivityPolicy &&
          other.version == version &&
          other.timezone == timezone &&
          other.weights == weights &&
          other.reason == reason &&
          other.changedBy == changedBy &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      version.hashCode +
      timezone.hashCode +
      weights.hashCode +
      reason.hashCode +
      changedBy.hashCode +
      createdAt.hashCode;

  factory ActivityPolicy.fromJson(Map<String, dynamic> json) =>
      _$ActivityPolicyFromJson(json);

  Map<String, dynamic> toJson() => _$ActivityPolicyToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ActivityPolicyTimezoneEnum {
  @JsonValue(r'Asia/Shanghai')
  asiaSlashShanghai(r'Asia/Shanghai'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ActivityPolicyTimezoneEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
