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
          'courseId',
          'teacherName',
          'weekday',
          'startSlot',
          'endSlot',
          'weeks',
          'location',
        ],
      );
      final val = TimeSlot(
        courseId: $checkedConvert('courseId', (v) => v as String),
        teacherName: $checkedConvert('teacherName', (v) => v as String?),
        weekday: $checkedConvert('weekday', (v) => (v as num).toInt()),
        startSlot: $checkedConvert('startSlot', (v) => (v as num).toInt()),
        endSlot: $checkedConvert('endSlot', (v) => (v as num).toInt()),
        weeks: $checkedConvert('weeks', (v) => v as String?),
        location: $checkedConvert('location', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$TimeSlotToJson(TimeSlot instance) => <String, dynamic>{
  'courseId': instance.courseId,
  'teacherName': instance.teacherName,
  'weekday': instance.weekday,
  'startSlot': instance.startSlot,
  'endSlot': instance.endSlot,
  'weeks': instance.weeks,
  'location': instance.location,
};
