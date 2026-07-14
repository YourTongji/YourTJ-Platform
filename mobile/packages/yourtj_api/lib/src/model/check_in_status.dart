//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'check_in_status.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CheckInStatus {
  /// Returns a new [CheckInStatus] instance.
  CheckInStatus({
    required this.timezone,

    required this.date,

    required this.checkedIn,

    required this.newlyCheckedIn,

    required this.checkedInAt,

    required this.currentStreak,

    required this.totalDays,

    required this.nextResetAt,
  });

  @JsonKey(
    name: r'timezone',
    required: true,
    includeIfNull: false,
    unknownEnumValue: CheckInStatusTimezoneEnum.unknownDefaultOpenApi,
  )
  final CheckInStatusTimezoneEnum timezone;

  @JsonKey(name: r'date', required: true, includeIfNull: false)
  final DateTime date;

  @JsonKey(name: r'checkedIn', required: true, includeIfNull: false)
  final bool checkedIn;

  /// True only when this request created today's check-in. Always false on GET.
  @JsonKey(name: r'newlyCheckedIn', required: true, includeIfNull: false)
  final bool newlyCheckedIn;

  /// Unix seconds for today's check-in, or null before check-in.
  @JsonKey(name: r'checkedInAt', required: true, includeIfNull: true)
  final int? checkedInAt;

  /// Consecutive checked days ending today, or yesterday before today's check-in.
  // minimum: 0
  @JsonKey(name: r'currentStreak', required: true, includeIfNull: false)
  final int currentStreak;

  // minimum: 0
  @JsonKey(name: r'totalDays', required: true, includeIfNull: false)
  final int totalDays;

  /// Unix seconds for the next Asia/Shanghai midnight.
  @JsonKey(name: r'nextResetAt', required: true, includeIfNull: false)
  final int nextResetAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CheckInStatus &&
          other.timezone == timezone &&
          other.date == date &&
          other.checkedIn == checkedIn &&
          other.newlyCheckedIn == newlyCheckedIn &&
          other.checkedInAt == checkedInAt &&
          other.currentStreak == currentStreak &&
          other.totalDays == totalDays &&
          other.nextResetAt == nextResetAt;

  @override
  int get hashCode =>
      timezone.hashCode +
      date.hashCode +
      checkedIn.hashCode +
      newlyCheckedIn.hashCode +
      (checkedInAt == null ? 0 : checkedInAt.hashCode) +
      currentStreak.hashCode +
      totalDays.hashCode +
      nextResetAt.hashCode;

  factory CheckInStatus.fromJson(Map<String, dynamic> json) =>
      _$CheckInStatusFromJson(json);

  Map<String, dynamic> toJson() => _$CheckInStatusToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum CheckInStatusTimezoneEnum {
  @JsonValue(r'Asia/Shanghai')
  asiaSlashShanghai(r'Asia/Shanghai'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const CheckInStatusTimezoneEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
