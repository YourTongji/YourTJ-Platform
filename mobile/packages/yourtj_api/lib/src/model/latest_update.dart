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
  LatestUpdate({
    required this.updatedAt,

    required this.importedAt,

    required this.stale,

    required this.staleAfterHours,
  });

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: true)
  final DateTime? updatedAt;

  /// Snapshot import time; never a substitute for upstream selection freshness.
  @JsonKey(name: r'importedAt', required: true, includeIfNull: true)
  final DateTime? importedAt;

  /// True when updatedAt is absent or older than staleAfterHours.
  @JsonKey(name: r'stale', required: true, includeIfNull: false)
  final bool stale;

  @JsonKey(
    name: r'staleAfterHours',
    required: true,
    includeIfNull: false,
    unknownEnumValue: LatestUpdateStaleAfterHoursEnum.unknownDefaultOpenApi,
  )
  final LatestUpdateStaleAfterHoursEnum staleAfterHours;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LatestUpdate &&
          other.updatedAt == updatedAt &&
          other.importedAt == importedAt &&
          other.stale == stale &&
          other.staleAfterHours == staleAfterHours;

  @override
  int get hashCode =>
      (updatedAt == null ? 0 : updatedAt.hashCode) +
      (importedAt == null ? 0 : importedAt.hashCode) +
      stale.hashCode +
      staleAfterHours.hashCode;

  factory LatestUpdate.fromJson(Map<String, dynamic> json) =>
      _$LatestUpdateFromJson(json);

  Map<String, dynamic> toJson() => _$LatestUpdateToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum LatestUpdateStaleAfterHoursEnum {
  @JsonValue(168)
  number168('168'),
  @JsonValue(11184809)
  unknownDefaultOpenApi('11184809');

  const LatestUpdateStaleAfterHoursEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
