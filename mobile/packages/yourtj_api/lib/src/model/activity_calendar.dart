//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/activity_weights.dart';
import 'package:yourtj_api/src/model/activity_day.dart';
import 'package:json_annotation/json_annotation.dart';

part 'activity_calendar.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ActivityCalendar {
  /// Returns a new [ActivityCalendar] instance.
  ActivityCalendar({
    required this.timezone,

    required this.from,

    required this.to,

    required this.policyVersion,

    required this.trustPolicyVersion,

    required this.weights,

    required this.likeDailyCap,

    required this.days,
  });

  @JsonKey(
    name: r'timezone',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ActivityCalendarTimezoneEnum.unknownDefaultOpenApi,
  )
  final ActivityCalendarTimezoneEnum timezone;

  @JsonKey(name: r'from', required: true, includeIfNull: false)
  final DateTime from;

  @JsonKey(name: r'to', required: true, includeIfNull: false)
  final DateTime to;

  /// Activity weight policy version.
  // minimum: 1
  @JsonKey(name: r'policyVersion', required: true, includeIfNull: false)
  final int policyVersion;

  /// Trust policy version supplying the daily like cap.
  // minimum: 1
  @JsonKey(name: r'trustPolicyVersion', required: true, includeIfNull: false)
  final int trustPolicyVersion;

  @JsonKey(name: r'weights', required: true, includeIfNull: false)
  final ActivityWeights weights;

  /// Maximum daily score contributed by positive likes.
  // minimum: 0
  @JsonKey(name: r'likeDailyCap', required: true, includeIfNull: false)
  final int likeDailyCap;

  @JsonKey(name: r'days', required: true, includeIfNull: false)
  final List<ActivityDay> days;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ActivityCalendar &&
          other.timezone == timezone &&
          other.from == from &&
          other.to == to &&
          other.policyVersion == policyVersion &&
          other.trustPolicyVersion == trustPolicyVersion &&
          other.weights == weights &&
          other.likeDailyCap == likeDailyCap &&
          other.days == days;

  @override
  int get hashCode =>
      timezone.hashCode +
      from.hashCode +
      to.hashCode +
      policyVersion.hashCode +
      trustPolicyVersion.hashCode +
      weights.hashCode +
      likeDailyCap.hashCode +
      days.hashCode;

  factory ActivityCalendar.fromJson(Map<String, dynamic> json) =>
      _$ActivityCalendarFromJson(json);

  Map<String, dynamic> toJson() => _$ActivityCalendarToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ActivityCalendarTimezoneEnum {
  @JsonValue(r'Asia/Shanghai')
  asiaSlashShanghai(r'Asia/Shanghai'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ActivityCalendarTimezoneEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
