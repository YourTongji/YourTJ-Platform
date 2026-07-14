//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'teacher.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Teacher {
  /// Returns a new [Teacher] instance.
  Teacher({this.id, this.name, this.title, this.department});

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'name', required: false, includeIfNull: false)
  final String? name;

  @JsonKey(name: r'title', required: false, includeIfNull: false)
  final String? title;

  @JsonKey(name: r'department', required: false, includeIfNull: false)
  final String? department;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Teacher &&
          other.id == id &&
          other.name == name &&
          other.title == title &&
          other.department == department;

  @override
  int get hashCode =>
      id.hashCode +
      name.hashCode +
      (title == null ? 0 : title.hashCode) +
      (department == null ? 0 : department.hashCode);

  factory Teacher.fromJson(Map<String, dynamic> json) =>
      _$TeacherFromJson(json);

  Map<String, dynamic> toJson() => _$TeacherToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
