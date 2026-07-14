//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'latest_update.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class LatestUpdate {
  /// Returns a new [LatestUpdate] instance.
  LatestUpdate({required this.updatedAt});

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: true)
  final DateTime? updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LatestUpdate && other.updatedAt == updatedAt;

  @override
  int get hashCode => (updatedAt == null ? 0 : updatedAt.hashCode);

  factory LatestUpdate.fromJson(Map<String, dynamic> json) =>
      _$LatestUpdateFromJson(json);

  Map<String, dynamic> toJson() => _$LatestUpdateToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
