//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'course_nature.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CourseNature {
  /// Returns a new [CourseNature] instance.
  CourseNature({required this.id, required this.name});

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CourseNature && other.id == id && other.name == name;

  @override
  int get hashCode => id.hashCode + name.hashCode;

  factory CourseNature.fromJson(Map<String, dynamic> json) =>
      _$CourseNatureFromJson(json);

  Map<String, dynamic> toJson() => _$CourseNatureToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
