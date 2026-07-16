// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'time_slot.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TimeSlot _$TimeSlotFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TimeSlot', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'offeringId',
          'courseId',
          'teacherName',
          'weekday',
          'startSlot',
          'endSlot',
          'weeks',
          'weekNumbers',
          'weeksUnknown',
          'location',
          'locationUnknown',
        ],
      );
      final val = TimeSlot(
        offeringId: $checkedConvert('offeringId', (v) => v as String),
        courseId: $checkedConvert('courseId', (v) => v as String),
        teacherName: $checkedConvert('teacherName', (v) => v as String?),
        weekday: $checkedConvert('weekday', (v) => (v as num).toInt()),
        startSlot: $checkedConvert('startSlot', (v) => (v as num).toInt()),
        endSlot: $checkedConvert('endSlot', (v) => (v as num).toInt()),
        weeks: $checkedConvert('weeks', (v) => v as String?),
        weekNumbers: $checkedConvert(
          'weekNumbers',
          (v) => (v as List<dynamic>).map((e) => (e as num).toInt()).toSet(),
        ),
        weeksUnknown: $checkedConvert('weeksUnknown', (v) => v as bool),
        location: $checkedConvert('location', (v) => v as String?),
        locationUnknown: $checkedConvert('locationUnknown', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$TimeSlotToJson(TimeSlot instance) => <String, dynamic>{
  'offeringId': instance.offeringId,
  'courseId': instance.courseId,
  'teacherName': instance.teacherName,
  'weekday': instance.weekday,
  'startSlot': instance.startSlot,
  'endSlot': instance.endSlot,
  'weeks': instance.weeks,
  'weekNumbers': instance.weekNumbers.toList(),
  'weeksUnknown': instance.weeksUnknown,
  'location': instance.location,
  'locationUnknown': instance.locationUnknown,
};
