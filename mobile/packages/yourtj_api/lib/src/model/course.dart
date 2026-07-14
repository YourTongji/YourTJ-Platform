//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'course.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Course {
  /// Returns a new [Course] instance.
  Course({
    this.id,

    this.code,

    this.name,

    this.credit,

    this.department,

    this.teacherName,

    this.reviewCount,

    this.reviewAvg,
  });

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'code', required: false, includeIfNull: false)
  final String? code;

  @JsonKey(name: r'name', required: false, includeIfNull: false)
  final String? name;

  @JsonKey(name: r'credit', required: false, includeIfNull: false)
  final num? credit;

  @JsonKey(name: r'department', required: false, includeIfNull: false)
  final String? department;

  @JsonKey(name: r'teacherName', required: false, includeIfNull: false)
  final String? teacherName;

  @JsonKey(name: r'reviewCount', required: false, includeIfNull: false)
  final int? reviewCount;

  @JsonKey(name: r'reviewAvg', required: false, includeIfNull: false)
  final num? reviewAvg;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Course &&
          other.id == id &&
          other.code == code &&
          other.name == name &&
          other.credit == credit &&
          other.department == department &&
          other.teacherName == teacherName &&
          other.reviewCount == reviewCount &&
          other.reviewAvg == reviewAvg;

  @override
  int get hashCode =>
      id.hashCode +
      code.hashCode +
      name.hashCode +
      (credit == null ? 0 : credit.hashCode) +
      (department == null ? 0 : department.hashCode) +
      (teacherName == null ? 0 : teacherName.hashCode) +
      reviewCount.hashCode +
      (reviewAvg == null ? 0 : reviewAvg.hashCode);

  factory Course.fromJson(Map<String, dynamic> json) => _$CourseFromJson(json);

  Map<String, dynamic> toJson() => _$CourseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
