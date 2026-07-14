//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'announcement_revision.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AnnouncementRevision {
  /// Returns a new [AnnouncementRevision] instance.
  AnnouncementRevision({
    required this.announcementId,

    required this.version,

    required this.revision,

    required this.title,

    this.body,

    required this.status,

    required this.presentation,

    required this.severity,

    required this.priority,

    required this.audience,

    required this.requiresAck,

    this.startsAt,

    this.endsAt,

    required this.createdAt,
  });

  @JsonKey(name: r'announcementId', required: true, includeIfNull: false)
  final String announcementId;

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  // minimum: 1
  @JsonKey(name: r'revision', required: true, includeIfNull: false)
  final int revision;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: false, includeIfNull: false)
  final String? body;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementRevisionStatusEnum.unknownDefaultOpenApi,
  )
  final AnnouncementRevisionStatusEnum status;

  @JsonKey(
    name: r'presentation',
    required: true,
    includeIfNull: false,
    unknownEnumValue:
        AnnouncementRevisionPresentationEnum.unknownDefaultOpenApi,
  )
  final AnnouncementRevisionPresentationEnum presentation;

  @JsonKey(
    name: r'severity',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementRevisionSeverityEnum.unknownDefaultOpenApi,
  )
  final AnnouncementRevisionSeverityEnum severity;

  @JsonKey(name: r'priority', required: true, includeIfNull: false)
  final int priority;

  @JsonKey(
    name: r'audience',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementRevisionAudienceEnum.unknownDefaultOpenApi,
  )
  final AnnouncementRevisionAudienceEnum audience;

  @JsonKey(name: r'requiresAck', required: true, includeIfNull: false)
  final bool requiresAck;

  @JsonKey(name: r'startsAt', required: false, includeIfNull: false)
  final int? startsAt;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AnnouncementRevision &&
          other.announcementId == announcementId &&
          other.version == version &&
          other.revision == revision &&
          other.title == title &&
          other.body == body &&
          other.status == status &&
          other.presentation == presentation &&
          other.severity == severity &&
          other.priority == priority &&
          other.audience == audience &&
          other.requiresAck == requiresAck &&
          other.startsAt == startsAt &&
          other.endsAt == endsAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      announcementId.hashCode +
      version.hashCode +
      revision.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      status.hashCode +
      presentation.hashCode +
      severity.hashCode +
      priority.hashCode +
      audience.hashCode +
      requiresAck.hashCode +
      (startsAt == null ? 0 : startsAt.hashCode) +
      (endsAt == null ? 0 : endsAt.hashCode) +
      createdAt.hashCode;

  factory AnnouncementRevision.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementRevisionFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementRevisionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AnnouncementRevisionStatusEnum {
  @JsonValue(r'draft')
  draft(r'draft'),
  @JsonValue(r'scheduled')
  scheduled(r'scheduled'),
  @JsonValue(r'published')
  published(r'published'),
  @JsonValue(r'archived')
  archived(r'archived'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementRevisionStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementRevisionPresentationEnum {
  @JsonValue(r'card')
  card(r'card'),
  @JsonValue(r'banner')
  banner(r'banner'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementRevisionPresentationEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementRevisionSeverityEnum {
  @JsonValue(r'info')
  info(r'info'),
  @JsonValue(r'success')
  success(r'success'),
  @JsonValue(r'warning')
  warning(r'warning'),
  @JsonValue(r'critical')
  critical(r'critical'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementRevisionSeverityEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementRevisionAudienceEnum {
  @JsonValue(r'all')
  all(r'all'),
  @JsonValue(r'authenticated')
  authenticated(r'authenticated'),
  @JsonValue(r'staff')
  staff(r'staff'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementRevisionAudienceEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
