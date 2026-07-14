// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'check_in_status.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CheckInStatus _$CheckInStatusFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('CheckInStatus', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'timezone',
      'date',
      'checkedIn',
      'newlyCheckedIn',
      'checkedInAt',
      'currentStreak',
      'totalDays',
      'nextResetAt',
    ],
  );
  final val = CheckInStatus(
    timezone: $checkedConvert(
      'timezone',
      (v) => $enumDecode(
        _$CheckInStatusTimezoneEnumEnumMap,
        v,
        unknownValue: CheckInStatusTimezoneEnum.unknownDefaultOpenApi,
      ),
    ),
    date: $checkedConvert('date', (v) => DateTime.parse(v as String)),
    checkedIn: $checkedConvert('checkedIn', (v) => v as bool),
    newlyCheckedIn: $checkedConvert('newlyCheckedIn', (v) => v as bool),
    checkedInAt: $checkedConvert('checkedInAt', (v) => (v as num?)?.toInt()),
    currentStreak: $checkedConvert('currentStreak', (v) => (v as num).toInt()),
    totalDays: $checkedConvert('totalDays', (v) => (v as num).toInt()),
    nextResetAt: $checkedConvert('nextResetAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$CheckInStatusToJson(CheckInStatus instance) =>
    <String, dynamic>{
      'timezone': _$CheckInStatusTimezoneEnumEnumMap[instance.timezone]!,
      'date': instance.date.toIso8601String(),
      'checkedIn': instance.checkedIn,
      'newlyCheckedIn': instance.newlyCheckedIn,
      'checkedInAt': instance.checkedInAt,
      'currentStreak': instance.currentStreak,
      'totalDays': instance.totalDays,
      'nextResetAt': instance.nextResetAt,
    };

const _$CheckInStatusTimezoneEnumEnumMap = {
  CheckInStatusTimezoneEnum.asiaSlashShanghai: 'Asia/Shanghai',
  CheckInStatusTimezoneEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
