//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'verification_revoke_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VerificationRevokeInput {
  /// Returns a new [VerificationRevokeInput] instance.
  VerificationRevokeInput({required this.reason});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VerificationRevokeInput && other.reason == reason;

  @override
  int get hashCode => reason.hashCode;

  factory VerificationRevokeInput.fromJson(Map<String, dynamic> json) =>
      _$VerificationRevokeInputFromJson(json);

  Map<String, dynamic> toJson() => _$VerificationRevokeInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
