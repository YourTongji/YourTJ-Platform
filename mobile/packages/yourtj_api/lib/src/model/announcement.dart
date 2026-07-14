//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/announcement_receipt_summary.dart';
import 'package:yourtj_api/src/model/announcement_receipt.dart';
import 'package:json_annotation/json_annotation.dart';

part 'announcement.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Announcement {
  /// Returns a new [Announcement] instance.
  Announcement({
    required this.id,

    required this.title,

    this.body,

    required this.status,

    required this.effectiveState,

    required this.presentation,

    required this.severity,

    required this.priority,

    required this.audience,

    required this.requiresAck,

    required this.version,

    required this.revision,

    this.startsAt,

    this.endsAt,

    this.publishedAt,

    this.archivedAt,

    required this.createdAt,

    required this.updatedAt,

    this.receipt,

    this.receiptSummary,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: false, includeIfNull: false)
  final String? body;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementStatusEnum.unknownDefaultOpenApi,
  )
  final AnnouncementStatusEnum status;

  @JsonKey(
    name: r'effectiveState',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementEffectiveStateEnum.unknownDefaultOpenApi,
  )
  final AnnouncementEffectiveStateEnum effectiveState;

  @JsonKey(
    name: r'presentation',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementPresentationEnum.unknownDefaultOpenApi,
  )
  final AnnouncementPresentationEnum presentation;

  @JsonKey(
    name: r'severity',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementSeverityEnum.unknownDefaultOpenApi,
  )
  final AnnouncementSeverityEnum severity;

  // minimum: -1000
  // maximum: 1000
  @JsonKey(name: r'priority', required: true, includeIfNull: false)
  final int priority;

  @JsonKey(
    name: r'audience',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementAudienceEnum.unknownDefaultOpenApi,
  )
  final AnnouncementAudienceEnum audience;

  @JsonKey(name: r'requiresAck', required: true, includeIfNull: false)
  final bool requiresAck;

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  // minimum: 1
  @JsonKey(name: r'revision', required: true, includeIfNull: false)
  final int revision;

  @JsonKey(name: r'startsAt', required: false, includeIfNull: false)
  final int? startsAt;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @JsonKey(name: r'publishedAt', required: false, includeIfNull: false)
  final int? publishedAt;

  @JsonKey(name: r'archivedAt', required: false, includeIfNull: false)
  final int? archivedAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @JsonKey(name: r'receipt', required: false, includeIfNull: false)
  final AnnouncementReceipt? receipt;

  @JsonKey(name: r'receiptSummary', required: false, includeIfNull: false)
  final AnnouncementReceiptSummary? receiptSummary;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Announcement &&
          other.id == id &&
          other.title == title &&
          other.body == body &&
          other.status == status &&
          other.effectiveState == effectiveState &&
          other.presentation == presentation &&
          other.severity == severity &&
          other.priority == priority &&
          other.audience == audience &&
          other.requiresAck == requiresAck &&
          other.version == version &&
          other.revision == revision &&
          other.startsAt == startsAt &&
          other.endsAt == endsAt &&
          other.publishedAt == publishedAt &&
          other.archivedAt == archivedAt &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt &&
          other.receipt == receipt &&
          other.receiptSummary == receiptSummary;

  @override
  int get hashCode =>
      id.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      status.hashCode +
      effectiveState.hashCode +
      presentation.hashCode +
      severity.hashCode +
      priority.hashCode +
      audience.hashCode +
      requiresAck.hashCode +
      version.hashCode +
      revision.hashCode +
      (startsAt == null ? 0 : startsAt.hashCode) +
      (endsAt == null ? 0 : endsAt.hashCode) +
      (publishedAt == null ? 0 : publishedAt.hashCode) +
      (archivedAt == null ? 0 : archivedAt.hashCode) +
      createdAt.hashCode +
      updatedAt.hashCode +
      (receipt == null ? 0 : receipt.hashCode) +
      (receiptSummary == null ? 0 : receiptSummary.hashCode);

  factory Announcement.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AnnouncementStatusEnum {
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

  const AnnouncementStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementEffectiveStateEnum {
  @JsonValue(r'draft')
  draft(r'draft'),
  @JsonValue(r'scheduled')
  scheduled(r'scheduled'),
  @JsonValue(r'active')
  active(r'active'),
  @JsonValue(r'expired')
  expired(r'expired'),
  @JsonValue(r'archived')
  archived(r'archived'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementEffectiveStateEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementPresentationEnum {
  @JsonValue(r'card')
  card(r'card'),
  @JsonValue(r'banner')
  banner(r'banner'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementPresentationEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementSeverityEnum {
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

  const AnnouncementSeverityEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementAudienceEnum {
  @JsonValue(r'all')
  all(r'all'),
  @JsonValue(r'authenticated')
  authenticated(r'authenticated'),
  @JsonValue(r'staff')
  staff(r'staff'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementAudienceEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
