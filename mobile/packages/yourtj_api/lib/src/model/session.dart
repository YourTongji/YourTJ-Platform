//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'session.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Session {
  /// Returns a new [Session] instance.
  Session({
    required this.id,

    required this.isCurrent,

    this.deviceLabel,

    required this.createdAt,

    required this.lastUsedAt,

    required this.expiresAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'isCurrent', required: true, includeIfNull: false)
  final bool isCurrent;

  @JsonKey(name: r'deviceLabel', required: false, includeIfNull: false)
  final String? deviceLabel;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'lastUsedAt', required: true, includeIfNull: false)
  final int lastUsedAt;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Session &&
          other.id == id &&
          other.isCurrent == isCurrent &&
          other.deviceLabel == deviceLabel &&
          other.createdAt == createdAt &&
          other.lastUsedAt == lastUsedAt &&
          other.expiresAt == expiresAt;

  @override
  int get hashCode =>
      id.hashCode +
      isCurrent.hashCode +
      (deviceLabel == null ? 0 : deviceLabel.hashCode) +
      createdAt.hashCode +
      lastUsedAt.hashCode +
      expiresAt.hashCode;

  factory Session.fromJson(Map<String, dynamic> json) =>
      _$SessionFromJson(json);

  Map<String, dynamic> toJson() => _$SessionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
