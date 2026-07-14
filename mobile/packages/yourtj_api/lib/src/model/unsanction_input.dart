//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'unsanction_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UnsanctionInput {
  /// Returns a new [UnsanctionInput] instance.
  UnsanctionInput({required this.sanctionId, required this.reason});

  @JsonKey(name: r'sanctionId', required: true, includeIfNull: false)
  final String sanctionId;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UnsanctionInput &&
          other.sanctionId == sanctionId &&
          other.reason == reason;

  @override
  int get hashCode => sanctionId.hashCode + reason.hashCode;

  factory UnsanctionInput.fromJson(Map<String, dynamic> json) =>
      _$UnsanctionInputFromJson(json);

  Map<String, dynamic> toJson() => _$UnsanctionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
