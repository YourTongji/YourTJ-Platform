//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'notification.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Notification {
  /// Returns a new [Notification] instance.
  Notification({
    required this.id,

    required this.type,

    required this.payload,

    required this.targetUrl,

    required this.read,

    required this.readAt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'type', required: true, includeIfNull: false)
  final String type;

  @JsonKey(name: r'payload', required: true, includeIfNull: false)
  final Map<String, Object> payload;

  /// Safe application-relative destination when this notification has one.
  @JsonKey(name: r'targetUrl', required: true, includeIfNull: true)
  final String? targetUrl;

  @JsonKey(name: r'read', required: true, includeIfNull: false)
  final bool read;

  @JsonKey(name: r'readAt', required: true, includeIfNull: true)
  final int? readAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Notification &&
          other.id == id &&
          other.type == type &&
          other.payload == payload &&
          other.targetUrl == targetUrl &&
          other.read == read &&
          other.readAt == readAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      type.hashCode +
      payload.hashCode +
      (targetUrl == null ? 0 : targetUrl.hashCode) +
      read.hashCode +
      (readAt == null ? 0 : readAt.hashCode) +
      createdAt.hashCode;

  factory Notification.fromJson(Map<String, dynamic> json) =>
      _$NotificationFromJson(json);

  Map<String, dynamic> toJson() => _$NotificationToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
