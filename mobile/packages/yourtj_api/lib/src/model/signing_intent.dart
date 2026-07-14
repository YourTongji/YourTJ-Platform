//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'signing_intent.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SigningIntent {
  /// Returns a new [SigningIntent] instance.
  SigningIntent({
    required this.intentId,

    required this.signingBytes,

    required this.expiresAt,
  });

  @JsonKey(name: r'intentId', required: true, includeIfNull: false)
  final String intentId;

  @JsonKey(name: r'signingBytes', required: true, includeIfNull: false)
  final String signingBytes;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SigningIntent &&
          other.intentId == intentId &&
          other.signingBytes == signingBytes &&
          other.expiresAt == expiresAt;

  @override
  int get hashCode =>
      intentId.hashCode + signingBytes.hashCode + expiresAt.hashCode;

  factory SigningIntent.fromJson(Map<String, dynamic> json) =>
      _$SigningIntentFromJson(json);

  Map<String, dynamic> toJson() => _$SigningIntentToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
