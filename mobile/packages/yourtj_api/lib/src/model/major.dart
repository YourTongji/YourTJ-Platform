//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'major.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Major {
  /// Returns a new [Major] instance.
  Major({
    required this.id,

    required this.name,

    required this.facultyId,

    required this.grade,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'facultyId', required: true, includeIfNull: true)
  final String? facultyId;

  @JsonKey(name: r'grade', required: true, includeIfNull: true)
  final String? grade;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Major &&
          other.id == id &&
          other.name == name &&
          other.facultyId == facultyId &&
          other.grade == grade;

  @override
  int get hashCode =>
      id.hashCode +
      name.hashCode +
      (facultyId == null ? 0 : facultyId.hashCode) +
      (grade == null ? 0 : grade.hashCode);

  factory Major.fromJson(Map<String, dynamic> json) => _$MajorFromJson(json);

  Map<String, dynamic> toJson() => _$MajorToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
