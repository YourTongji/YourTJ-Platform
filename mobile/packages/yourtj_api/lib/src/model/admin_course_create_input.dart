//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_course_create_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminCourseCreateInput {
  /// Returns a new [AdminCourseCreateInput] instance.
  AdminCourseCreateInput({
    required this.code,

    required this.name,

    this.credit,

    this.department,

    this.teacherName,

    required this.reason,
  });

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  // minimum: 0
  // maximum: 100
  @JsonKey(name: r'credit', required: false, includeIfNull: false)
  final num? credit;

  @JsonKey(name: r'department', required: false, includeIfNull: false)
  final String? department;

  @JsonKey(name: r'teacherName', required: false, includeIfNull: false)
  final String? teacherName;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminCourseCreateInput &&
          other.code == code &&
          other.name == name &&
          other.credit == credit &&
          other.department == department &&
          other.teacherName == teacherName &&
          other.reason == reason;

  @override
  int get hashCode =>
      code.hashCode +
      name.hashCode +
      credit.hashCode +
      department.hashCode +
      teacherName.hashCode +
      reason.hashCode;

  factory AdminCourseCreateInput.fromJson(Map<String, dynamic> json) =>
      _$AdminCourseCreateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminCourseCreateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
