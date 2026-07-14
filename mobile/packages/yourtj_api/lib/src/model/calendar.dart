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
  Calendar({this.id, this.name, this.isCurrent});

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'name', required: false, includeIfNull: false)
  final String? name;

  @JsonKey(name: r'isCurrent', required: false, includeIfNull: false)
  final bool? isCurrent;

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
