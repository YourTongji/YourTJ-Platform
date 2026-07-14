//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'appeal_access_token.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AppealAccessToken {
  /// Returns a new [AppealAccessToken] instance.
  AppealAccessToken({required this.accessToken, required this.expiresAt});

  /// Short-lived bearer accepted only by /me/appeals and /me/governance-notices.
  @JsonKey(name: r'accessToken', required: true, includeIfNull: false)
  final String accessToken;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AppealAccessToken &&
          other.accessToken == accessToken &&
          other.expiresAt == expiresAt;

  @override
  int get hashCode => accessToken.hashCode + expiresAt.hashCode;

  factory AppealAccessToken.fromJson(Map<String, dynamic> json) =>
      _$AppealAccessTokenFromJson(json);

  Map<String, dynamic> toJson() => _$AppealAccessTokenToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
