//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'selection_course.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SelectionCourse {
  /// Returns a new [SelectionCourse] instance.
  SelectionCourse({
    required this.id,

    required this.code,

    required this.name,

    required this.credit,

    required this.natureId,

    required this.campusId,

    required this.teacherName,

    required this.teacherNames,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'credit', required: true, includeIfNull: true)
  final num? credit;

  @JsonKey(name: r'natureId', required: true, includeIfNull: true)
  final String? natureId;

  @JsonKey(name: r'campusId', required: true, includeIfNull: true)
  final String? campusId;

  @JsonKey(name: r'teacherName', required: true, includeIfNull: true)
  final String? teacherName;

  @JsonKey(name: r'teacherNames', required: true, includeIfNull: false)
  final List<String> teacherNames;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SelectionCourse &&
          other.id == id &&
          other.code == code &&
          other.name == name &&
          other.credit == credit &&
          other.natureId == natureId &&
          other.campusId == campusId &&
          other.teacherName == teacherName &&
          other.teacherNames == teacherNames;

  @override
  int get hashCode =>
      id.hashCode +
      code.hashCode +
      name.hashCode +
      (credit == null ? 0 : credit.hashCode) +
      (natureId == null ? 0 : natureId.hashCode) +
      (campusId == null ? 0 : campusId.hashCode) +
      (teacherName == null ? 0 : teacherName.hashCode) +
      teacherNames.hashCode;

  factory SelectionCourse.fromJson(Map<String, dynamic> json) =>
      _$SelectionCourseFromJson(json);

  Map<String, dynamic> toJson() => _$SelectionCourseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
