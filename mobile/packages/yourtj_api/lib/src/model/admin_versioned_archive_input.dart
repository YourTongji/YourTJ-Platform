//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_versioned_archive_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminVersionedArchiveInput {
  /// Returns a new [AdminVersionedArchiveInput] instance.
  AdminVersionedArchiveInput({
    required this.expectedVersion,

    required this.reason,
  });

  // minimum: 1
  @JsonKey(name: r'expectedVersion', required: true, includeIfNull: false)
  final int expectedVersion;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminVersionedArchiveInput &&
          other.expectedVersion == expectedVersion &&
          other.reason == reason;

  @override
  int get hashCode => expectedVersion.hashCode + reason.hashCode;

  factory AdminVersionedArchiveInput.fromJson(Map<String, dynamic> json) =>
      _$AdminVersionedArchiveInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminVersionedArchiveInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
