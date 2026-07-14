//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'faculty.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Faculty {
  /// Returns a new [Faculty] instance.
  Faculty({this.id, this.name, this.campusId});

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'name', required: false, includeIfNull: false)
  final String? name;

  @JsonKey(name: r'campusId', required: false, includeIfNull: false)
  final String? campusId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Faculty &&
          other.id == id &&
          other.name == name &&
          other.campusId == campusId;

  @override
  int get hashCode =>
      id.hashCode + name.hashCode + (campusId == null ? 0 : campusId.hashCode);

  factory Faculty.fromJson(Map<String, dynamic> json) =>
      _$FacultyFromJson(json);

  Map<String, dynamic> toJson() => _$FacultyToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
