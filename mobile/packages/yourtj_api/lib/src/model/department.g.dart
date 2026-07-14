// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'department.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Department _$DepartmentFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Department', json, ($checkedConvert) {
      final val = Department(
        id: $checkedConvert('id', (v) => v as String?),
        name: $checkedConvert('name', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$DepartmentToJson(Department instance) =>
    <String, dynamic>{'id': ?instance.id, 'name': ?instance.name};
