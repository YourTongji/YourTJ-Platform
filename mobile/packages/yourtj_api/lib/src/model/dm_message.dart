//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_message.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmMessage {
  /// Returns a new [DmMessage] instance.
  DmMessage({
    required this.id,

    required this.conversationId,

    required this.senderId,

    required this.senderHandle,

    this.senderDisplayName,

    required this.body,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'conversationId', required: true, includeIfNull: false)
  final String conversationId;

  @JsonKey(name: r'senderId', required: true, includeIfNull: false)
  final String senderId;

  @JsonKey(name: r'senderHandle', required: true, includeIfNull: false)
  final String senderHandle;

  @JsonKey(name: r'senderDisplayName', required: false, includeIfNull: false)
  final String? senderDisplayName;

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmMessage &&
          other.id == id &&
          other.conversationId == conversationId &&
          other.senderId == senderId &&
          other.senderHandle == senderHandle &&
          other.senderDisplayName == senderDisplayName &&
          other.body == body &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      conversationId.hashCode +
      senderId.hashCode +
      senderHandle.hashCode +
      (senderDisplayName == null ? 0 : senderDisplayName.hashCode) +
      body.hashCode +
      createdAt.hashCode;

  factory DmMessage.fromJson(Map<String, dynamic> json) =>
      _$DmMessageFromJson(json);

  Map<String, dynamic> toJson() => _$DmMessageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
