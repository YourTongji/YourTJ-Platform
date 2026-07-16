//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'signing_intent_outcome.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SigningIntentOutcome {
  /// Returns a new [SigningIntentOutcome] instance.
  SigningIntentOutcome({
    required this.intentId,

    required this.status,

    required this.expiresAt,
  });

  @JsonKey(name: r'intentId', required: true, includeIfNull: false)
  final String intentId;

  /// Committed is visible only after the consuming business transaction commits; expired means no consumer holds the intent row lock and the intent was not consumed before expiry.
  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SigningIntentOutcomeStatusEnum.unknownDefaultOpenApi,
  )
  final SigningIntentOutcomeStatusEnum status;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SigningIntentOutcome &&
          other.intentId == intentId &&
          other.status == status &&
          other.expiresAt == expiresAt;

  @override
  int get hashCode => intentId.hashCode + status.hashCode + expiresAt.hashCode;

  factory SigningIntentOutcome.fromJson(Map<String, dynamic> json) =>
      _$SigningIntentOutcomeFromJson(json);

  Map<String, dynamic> toJson() => _$SigningIntentOutcomeToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

/// Committed is visible only after the consuming business transaction commits; expired means no consumer holds the intent row lock and the intent was not consumed before expiry.
enum SigningIntentOutcomeStatusEnum {
  /// Committed is visible only after the consuming business transaction commits; expired means no consumer holds the intent row lock and the intent was not consumed before expiry.
  @JsonValue(r'pending')
  pending(r'pending'),

  /// Committed is visible only after the consuming business transaction commits; expired means no consumer holds the intent row lock and the intent was not consumed before expiry.
  @JsonValue(r'committed')
  committed(r'committed'),

  /// Committed is visible only after the consuming business transaction commits; expired means no consumer holds the intent row lock and the intent was not consumed before expiry.
  @JsonValue(r'expired')
  expired(r'expired'),

  /// Committed is visible only after the consuming business transaction commits; expired means no consumer holds the intent row lock and the intent was not consumed before expiry.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SigningIntentOutcomeStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
