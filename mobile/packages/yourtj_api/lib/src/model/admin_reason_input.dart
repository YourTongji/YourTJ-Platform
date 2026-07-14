//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_reason_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminReasonInput {
  /// Returns a new [AdminReasonInput] instance.
  AdminReasonInput({required this.reason});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminReasonInput && other.reason == reason;

  @override
  int get hashCode => reason.hashCode;

  factory AdminReasonInput.fromJson(Map<String, dynamic> json) =>
      _$AdminReasonInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminReasonInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
