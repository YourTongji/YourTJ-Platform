//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/recent_auth_method.dart';
import 'package:json_annotation/json_annotation.dart';

part 'recent_auth_verify_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class RecentAuthVerifyInput {
  /// Returns a new [RecentAuthVerifyInput] instance.
  RecentAuthVerifyInput({required this.method, this.password, this.code});

  @JsonKey(
    name: r'method',
    required: true,
    includeIfNull: false,
    unknownEnumValue: RecentAuthMethod.unknownDefaultOpenApi,
  )
  final RecentAuthMethod method;

  @JsonKey(name: r'password', required: false, includeIfNull: false)
  final String? password;

  @JsonKey(name: r'code', required: false, includeIfNull: false)
  final String? code;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RecentAuthVerifyInput &&
          other.method == method &&
          other.password == password &&
          other.code == code;

  @override
  int get hashCode => method.hashCode + password.hashCode + code.hashCode;

  factory RecentAuthVerifyInput.fromJson(Map<String, dynamic> json) =>
      _$RecentAuthVerifyInputFromJson(json);

  Map<String, dynamic> toJson() => _$RecentAuthVerifyInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
