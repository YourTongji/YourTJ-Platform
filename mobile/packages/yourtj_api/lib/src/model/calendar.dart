//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'calendar.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Calendar {
  /// Returns a new [Calendar] instance.
  Calendar({required this.id, required this.name, required this.isCurrent});

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'isCurrent', required: true, includeIfNull: false)
  final bool isCurrent;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Calendar &&
          other.id == id &&
          other.name == name &&
          other.isCurrent == isCurrent;

  @override
  int get hashCode => id.hashCode + name.hashCode + isCurrent.hashCode;

  factory Calendar.fromJson(Map<String, dynamic> json) =>
      _$CalendarFromJson(json);

  Map<String, dynamic> toJson() => _$CalendarToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
