// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'faculty.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Faculty _$FacultyFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Faculty', json, ($checkedConvert) {
      final val = Faculty(
        id: $checkedConvert('id', (v) => v as String?),
        name: $checkedConvert('name', (v) => v as String?),
        campusId: $checkedConvert('campusId', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$FacultyToJson(Faculty instance) => <String, dynamic>{
  'id': ?instance.id,
  'name': ?instance.name,
  'campusId': ?instance.campusId,
};
