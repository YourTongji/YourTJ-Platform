//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_forum_flag.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminForumFlag {
  /// Returns a new [AdminForumFlag] instance.
  AdminForumFlag({
    required this.id,

    required this.targetType,

    required this.targetId,

    required this.reporterId,

    required this.reason,

    this.note,

    required this.weight,

    required this.status,

    this.handledBy,

    this.handledAt,

    this.resolutionNote,

    this.authorHandle,

    this.targetTitle,

    this.contentExcerpt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminForumFlagTargetTypeEnum.unknownDefaultOpenApi,
  )
  final AdminForumFlagTargetTypeEnum targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @JsonKey(name: r'reporterId', required: true, includeIfNull: false)
  final String reporterId;

  @JsonKey(
    name: r'reason',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminForumFlagReasonEnum.unknownDefaultOpenApi,
  )
  final AdminForumFlagReasonEnum reason;

  @JsonKey(name: r'note', required: false, includeIfNull: false)
  final String? note;

  @JsonKey(name: r'weight', required: true, includeIfNull: false)
  final num weight;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminForumFlagStatusEnum.unknownDefaultOpenApi,
  )
  final AdminForumFlagStatusEnum status;

  @JsonKey(name: r'handledBy', required: false, includeIfNull: false)
  final String? handledBy;

  @JsonKey(name: r'handledAt', required: false, includeIfNull: false)
  final int? handledAt;

  @JsonKey(name: r'resolutionNote', required: false, includeIfNull: false)
  final String? resolutionNote;

  @JsonKey(name: r'authorHandle', required: false, includeIfNull: false)
  final String? authorHandle;

  @JsonKey(name: r'targetTitle', required: false, includeIfNull: false)
  final String? targetTitle;

  @JsonKey(name: r'contentExcerpt', required: false, includeIfNull: false)
  final String? contentExcerpt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminForumFlag &&
          other.id == id &&
          other.targetType == targetType &&
          other.targetId == targetId &&
          other.reporterId == reporterId &&
          other.reason == reason &&
          other.note == note &&
          other.weight == weight &&
          other.status == status &&
          other.handledBy == handledBy &&
          other.handledAt == handledAt &&
          other.resolutionNote == resolutionNote &&
          other.authorHandle == authorHandle &&
          other.targetTitle == targetTitle &&
          other.contentExcerpt == contentExcerpt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      targetType.hashCode +
      targetId.hashCode +
      reporterId.hashCode +
      reason.hashCode +
      (note == null ? 0 : note.hashCode) +
      weight.hashCode +
      status.hashCode +
      (handledBy == null ? 0 : handledBy.hashCode) +
      (handledAt == null ? 0 : handledAt.hashCode) +
      (resolutionNote == null ? 0 : resolutionNote.hashCode) +
      (authorHandle == null ? 0 : authorHandle.hashCode) +
      (targetTitle == null ? 0 : targetTitle.hashCode) +
      (contentExcerpt == null ? 0 : contentExcerpt.hashCode) +
      createdAt.hashCode;

  factory AdminForumFlag.fromJson(Map<String, dynamic> json) =>
      _$AdminForumFlagFromJson(json);

  Map<String, dynamic> toJson() => _$AdminForumFlagToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AdminForumFlagTargetTypeEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminForumFlagTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AdminForumFlagReasonEnum {
  @JsonValue(r'spam')
  spam(r'spam'),
  @JsonValue(r'abuse')
  abuse(r'abuse'),
  @JsonValue(r'off_topic')
  offTopic(r'off_topic'),
  @JsonValue(r'illegal')
  illegal(r'illegal'),
  @JsonValue(r'other')
  other(r'other'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminForumFlagReasonEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AdminForumFlagStatusEnum {
  @JsonValue(r'open')
  open(r'open'),
  @JsonValue(r'upheld')
  upheld(r'upheld'),
  @JsonValue(r'rejected')
  rejected(r'rejected'),
  @JsonValue(r'ignored')
  ignored(r'ignored'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminForumFlagStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
