// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'activity_calendar.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ActivityCalendar _$ActivityCalendarFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ActivityCalendar', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'timezone',
      'from',
      'to',
      'policyVersion',
      'trustPolicyVersion',
      'weights',
      'likeDailyCap',
      'days',
    ],
  );
  final val = ActivityCalendar(
    timezone: $checkedConvert(
      'timezone',
      (v) => $enumDecode(
        _$ActivityCalendarTimezoneEnumEnumMap,
        v,
        unknownValue: ActivityCalendarTimezoneEnum.unknownDefaultOpenApi,
      ),
    ),
    from: $checkedConvert('from', (v) => DateTime.parse(v as String)),
    to: $checkedConvert('to', (v) => DateTime.parse(v as String)),
    policyVersion: $checkedConvert('policyVersion', (v) => (v as num).toInt()),
    trustPolicyVersion: $checkedConvert(
      'trustPolicyVersion',
      (v) => (v as num).toInt(),
    ),
    weights: $checkedConvert(
      'weights',
      (v) => ActivityWeights.fromJson(v as Map<String, dynamic>),
    ),
    likeDailyCap: $checkedConvert('likeDailyCap', (v) => (v as num).toInt()),
    days: $checkedConvert(
      'days',
      (v) => (v as List<dynamic>)
          .map((e) => ActivityDay.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
  );
  return val;
});

Map<String, dynamic> _$ActivityCalendarToJson(ActivityCalendar instance) =>
    <String, dynamic>{
      'timezone': _$ActivityCalendarTimezoneEnumEnumMap[instance.timezone]!,
      'from': instance.from.toIso8601String(),
      'to': instance.to.toIso8601String(),
      'policyVersion': instance.policyVersion,
      'trustPolicyVersion': instance.trustPolicyVersion,
      'weights': instance.weights.toJson(),
      'likeDailyCap': instance.likeDailyCap,
      'days': instance.days.map((e) => e.toJson()).toList(),
    };

const _$ActivityCalendarTimezoneEnumEnumMap = {
  ActivityCalendarTimezoneEnum.asiaSlashShanghai: 'Asia/Shanghai',
  ActivityCalendarTimezoneEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
