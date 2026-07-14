//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_user_role_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminUserRoleInput {
  /// Returns a new [AdminUserRoleInput] instance.
  AdminUserRoleInput({required this.role, required this.reason});

  @JsonKey(
    name: r'role',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminUserRoleInputRoleEnum.unknownDefaultOpenApi,
  )
  final AdminUserRoleInputRoleEnum role;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminUserRoleInput &&
          other.role == role &&
          other.reason == reason;

  @override
  int get hashCode => role.hashCode + reason.hashCode;

  factory AdminUserRoleInput.fromJson(Map<String, dynamic> json) =>
      _$AdminUserRoleInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminUserRoleInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AdminUserRoleInputRoleEnum {
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'mod')
  mod(r'mod'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminUserRoleInputRoleEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
