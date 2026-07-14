//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_conversation.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmConversation {
  /// Returns a new [DmConversation] instance.
  DmConversation({
    required this.id,

    required this.participantId,

    required this.participantHandle,

    this.participantDisplayName,

    this.participantAvatarUrl,

    this.lastMessageExcerpt,

    this.lastMessageAt,

    required this.unreadCount,

    required this.isArchived,

    required this.isMuted,

    required this.isDeleted,

    required this.requestStatus,

    this.requestDirection,

    required this.canSend,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'participantId', required: true, includeIfNull: false)
  final String participantId;

  @JsonKey(name: r'participantHandle', required: true, includeIfNull: false)
  final String participantHandle;

  @JsonKey(
    name: r'participantDisplayName',
    required: false,
    includeIfNull: false,
  )
  final String? participantDisplayName;

  /// Short-lived clean thumb_256 compatibility URL; refresh the owning conversation response after expiry.
  @JsonKey(name: r'participantAvatarUrl', required: false, includeIfNull: false)
  final String? participantAvatarUrl;

  @JsonKey(name: r'lastMessageExcerpt', required: false, includeIfNull: false)
  final String? lastMessageExcerpt;

  @JsonKey(name: r'lastMessageAt', required: false, includeIfNull: false)
  final int? lastMessageAt;

  // minimum: 0
  @JsonKey(name: r'unreadCount', required: true, includeIfNull: false)
  final int unreadCount;

  @JsonKey(name: r'isArchived', required: true, includeIfNull: false)
  final bool isArchived;

  @JsonKey(name: r'isMuted', required: true, includeIfNull: false)
  final bool isMuted;

  @JsonKey(name: r'isDeleted', required: true, includeIfNull: false)
  final bool isDeleted;

  @JsonKey(
    name: r'requestStatus',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DmConversationRequestStatusEnum.unknownDefaultOpenApi,
  )
  final DmConversationRequestStatusEnum requestStatus;

  @JsonKey(
    name: r'requestDirection',
    required: false,
    includeIfNull: false,
    unknownEnumValue: DmConversationRequestDirectionEnum.unknownDefaultOpenApi,
  )
  final DmConversationRequestDirectionEnum? requestDirection;

  /// False while a one-message request awaits acceptance.
  @JsonKey(name: r'canSend', required: true, includeIfNull: false)
  final bool canSend;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmConversation &&
          other.id == id &&
          other.participantId == participantId &&
          other.participantHandle == participantHandle &&
          other.participantDisplayName == participantDisplayName &&
          other.participantAvatarUrl == participantAvatarUrl &&
          other.lastMessageExcerpt == lastMessageExcerpt &&
          other.lastMessageAt == lastMessageAt &&
          other.unreadCount == unreadCount &&
          other.isArchived == isArchived &&
          other.isMuted == isMuted &&
          other.isDeleted == isDeleted &&
          other.requestStatus == requestStatus &&
          other.requestDirection == requestDirection &&
          other.canSend == canSend &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      participantId.hashCode +
      participantHandle.hashCode +
      (participantDisplayName == null ? 0 : participantDisplayName.hashCode) +
      (participantAvatarUrl == null ? 0 : participantAvatarUrl.hashCode) +
      (lastMessageExcerpt == null ? 0 : lastMessageExcerpt.hashCode) +
      (lastMessageAt == null ? 0 : lastMessageAt.hashCode) +
      unreadCount.hashCode +
      isArchived.hashCode +
      isMuted.hashCode +
      isDeleted.hashCode +
      requestStatus.hashCode +
      (requestDirection == null ? 0 : requestDirection.hashCode) +
      canSend.hashCode +
      createdAt.hashCode;

  factory DmConversation.fromJson(Map<String, dynamic> json) =>
      _$DmConversationFromJson(json);

  Map<String, dynamic> toJson() => _$DmConversationToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum DmConversationRequestStatusEnum {
  @JsonValue(r'accepted')
  accepted(r'accepted'),
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmConversationRequestStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum DmConversationRequestDirectionEnum {
  @JsonValue(r'incoming')
  incoming(r'incoming'),
  @JsonValue(r'outgoing')
  outgoing(r'outgoing'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmConversationRequestDirectionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
