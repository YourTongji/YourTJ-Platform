//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_user_invite_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminUserInviteInput {
  /// Returns a new [AdminUserInviteInput] instance.
  AdminUserInviteInput({
    required this.email,

    required this.handle,

    required this.reason,
  });

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminUserInviteInput &&
          other.email == email &&
          other.handle == handle &&
          other.reason == reason;

  @override
  int get hashCode => email.hashCode + handle.hashCode + reason.hashCode;

  factory AdminUserInviteInput.fromJson(Map<String, dynamic> json) =>
      _$AdminUserInviteInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminUserInviteInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
