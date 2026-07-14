//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'refresh_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class RefreshInput {
  /// Returns a new [RefreshInput] instance.
  RefreshInput({required this.refreshToken});

  @JsonKey(name: r'refreshToken', required: true, includeIfNull: false)
  final String refreshToken;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RefreshInput && other.refreshToken == refreshToken;

  @override
  int get hashCode => refreshToken.hashCode;

  factory RefreshInput.fromJson(Map<String, dynamic> json) =>
      _$RefreshInputFromJson(json);

  Map<String, dynamic> toJson() => _$RefreshInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
