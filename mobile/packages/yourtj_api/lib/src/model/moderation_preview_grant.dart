//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'moderation_preview_grant.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ModerationPreviewGrant {
  /// Returns a new [ModerationPreviewGrant] instance.
  ModerationPreviewGrant({required this.token, required this.expiresAt});

  @JsonKey(name: r'token', required: true, includeIfNull: false)
  final String token;

  /// Unix seconds; grants expire after 60 seconds and are consumed once.
  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ModerationPreviewGrant &&
          other.token == token &&
          other.expiresAt == expiresAt;

  @override
  int get hashCode => token.hashCode + expiresAt.hashCode;

  factory ModerationPreviewGrant.fromJson(Map<String, dynamic> json) =>
      _$ModerationPreviewGrantFromJson(json);

  Map<String, dynamic> toJson() => _$ModerationPreviewGrantToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
