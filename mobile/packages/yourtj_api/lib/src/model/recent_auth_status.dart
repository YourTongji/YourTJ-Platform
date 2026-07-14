//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/recent_auth_method.dart';
import 'package:json_annotation/json_annotation.dart';

part 'recent_auth_status.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class RecentAuthStatus {
  /// Returns a new [RecentAuthStatus] instance.
  RecentAuthStatus({
    required this.sessionBound,

    required this.isFresh,

    required this.authenticatedAt,

    required this.expiresAt,

    required this.method,

    required this.availableMethods,
  });

  /// False for rolling-window legacy JWTs; high-risk mutations fail closed until a new session login.
  @JsonKey(name: r'sessionBound', required: true, includeIfNull: false)
  final bool sessionBound;

  @JsonKey(name: r'isFresh', required: true, includeIfNull: false)
  final bool isFresh;

  @JsonKey(name: r'authenticatedAt', required: true, includeIfNull: true)
  final int? authenticatedAt;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: true)
  final int? expiresAt;

  @JsonKey(
    name: r'method',
    required: true,
    includeIfNull: true,
    unknownEnumValue: RecentAuthMethod.unknownDefaultOpenApi,
  )
  final RecentAuthMethod? method;

  /// Password appears only when the authenticated account has one; email_code never accepts a client-supplied email.
  @JsonKey(name: r'availableMethods', required: true, includeIfNull: false)
  final List<RecentAuthMethod> availableMethods;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RecentAuthStatus &&
          other.sessionBound == sessionBound &&
          other.isFresh == isFresh &&
          other.authenticatedAt == authenticatedAt &&
          other.expiresAt == expiresAt &&
          other.method == method &&
          other.availableMethods == availableMethods;

  @override
  int get hashCode =>
      sessionBound.hashCode +
      isFresh.hashCode +
      (authenticatedAt == null ? 0 : authenticatedAt.hashCode) +
      (expiresAt == null ? 0 : expiresAt.hashCode) +
      (method == null ? 0 : method.hashCode) +
      availableMethods.hashCode;

  factory RecentAuthStatus.fromJson(Map<String, dynamic> json) =>
      _$RecentAuthStatusFromJson(json);

  Map<String, dynamic> toJson() => _$RecentAuthStatusToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
