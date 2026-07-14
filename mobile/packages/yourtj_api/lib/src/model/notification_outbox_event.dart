//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'notification_outbox_event.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class NotificationOutboxEvent {
  /// Returns a new [NotificationOutboxEvent] instance.
  NotificationOutboxEvent({
    required this.id,

    required this.topic,

    required this.recipientAccountId,

    required this.eventType,

    required this.state,

    required this.attempts,

    required this.maxAttempts,

    required this.manualRetryCount,

    required this.availableAt,

    this.lastErrorCode,

    this.completedAt,

    this.deadAt,

    required this.createdAt,

    required this.updatedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'topic',
    required: true,
    includeIfNull: false,
    unknownEnumValue: NotificationOutboxEventTopicEnum.unknownDefaultOpenApi,
  )
  final NotificationOutboxEventTopicEnum topic;

  @JsonKey(name: r'recipientAccountId', required: true, includeIfNull: false)
  final String recipientAccountId;

  @JsonKey(name: r'eventType', required: true, includeIfNull: false)
  final String eventType;

  @JsonKey(
    name: r'state',
    required: true,
    includeIfNull: false,
    unknownEnumValue: NotificationOutboxEventStateEnum.unknownDefaultOpenApi,
  )
  final NotificationOutboxEventStateEnum state;

  // minimum: 0
  @JsonKey(name: r'attempts', required: true, includeIfNull: false)
  final int attempts;

  // minimum: 1
  @JsonKey(name: r'maxAttempts', required: true, includeIfNull: false)
  final int maxAttempts;

  // minimum: 0
  @JsonKey(name: r'manualRetryCount', required: true, includeIfNull: false)
  final int manualRetryCount;

  @JsonKey(name: r'availableAt', required: true, includeIfNull: false)
  final int availableAt;

  @JsonKey(name: r'lastErrorCode', required: false, includeIfNull: false)
  final String? lastErrorCode;

  @JsonKey(name: r'completedAt', required: false, includeIfNull: false)
  final int? completedAt;

  @JsonKey(name: r'deadAt', required: false, includeIfNull: false)
  final int? deadAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is NotificationOutboxEvent &&
          other.id == id &&
          other.topic == topic &&
          other.recipientAccountId == recipientAccountId &&
          other.eventType == eventType &&
          other.state == state &&
          other.attempts == attempts &&
          other.maxAttempts == maxAttempts &&
          other.manualRetryCount == manualRetryCount &&
          other.availableAt == availableAt &&
          other.lastErrorCode == lastErrorCode &&
          other.completedAt == completedAt &&
          other.deadAt == deadAt &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      id.hashCode +
      topic.hashCode +
      recipientAccountId.hashCode +
      eventType.hashCode +
      state.hashCode +
      attempts.hashCode +
      maxAttempts.hashCode +
      manualRetryCount.hashCode +
      availableAt.hashCode +
      (lastErrorCode == null ? 0 : lastErrorCode.hashCode) +
      (completedAt == null ? 0 : completedAt.hashCode) +
      (deadAt == null ? 0 : deadAt.hashCode) +
      createdAt.hashCode +
      updatedAt.hashCode;

  factory NotificationOutboxEvent.fromJson(Map<String, dynamic> json) =>
      _$NotificationOutboxEventFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationOutboxEventToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum NotificationOutboxEventTopicEnum {
  @JsonValue(r'notification')
  notification(r'notification'),
  @JsonValue(r'achievement_award')
  achievementAward(r'achievement_award'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const NotificationOutboxEventTopicEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum NotificationOutboxEventStateEnum {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'running')
  running(r'running'),
  @JsonValue(r'succeeded')
  succeeded(r'succeeded'),
  @JsonValue(r'dead')
  dead(r'dead'),
  @JsonValue(r'cancelled')
  cancelled(r'cancelled'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const NotificationOutboxEventStateEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
