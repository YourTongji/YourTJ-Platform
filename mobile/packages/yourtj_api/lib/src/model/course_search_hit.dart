//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'course_search_hit.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CourseSearchHit {
  /// Returns a new [CourseSearchHit] instance.
  CourseSearchHit({
    required this.id,

    required this.code,

    required this.name,

    required this.credit,

    required this.department,

    required this.teacherName,

    required this.reviewCount,

    required this.reviewAvg,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'credit', required: true, includeIfNull: true)
  final num? credit;

  @JsonKey(name: r'department', required: true, includeIfNull: true)
  final String? department;

  @JsonKey(name: r'teacherName', required: true, includeIfNull: true)
  final String? teacherName;

  // minimum: 0
  @JsonKey(name: r'reviewCount', required: true, includeIfNull: false)
  final int reviewCount;

  @JsonKey(name: r'reviewAvg', required: true, includeIfNull: true)
  final num? reviewAvg;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CourseSearchHit &&
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

  factory CourseSearchHit.fromJson(Map<String, dynamic> json) =>
      _$CourseSearchHitFromJson(json);

  Map<String, dynamic> toJson() => _$CourseSearchHitToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
