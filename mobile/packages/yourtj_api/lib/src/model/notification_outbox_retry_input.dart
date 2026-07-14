//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'notification_outbox_retry_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationOutboxRetryInput {
  /// Returns a new [NotificationOutboxRetryInput] instance.
  NotificationOutboxRetryInput({required this.reason});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationOutboxRetryInput && other.reason == reason;

  @override
  int get hashCode => reason.hashCode;

  factory NotificationOutboxRetryInput.fromJson(Map<String, dynamic> json) =>
      _$NotificationOutboxRetryInputFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationOutboxRetryInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
