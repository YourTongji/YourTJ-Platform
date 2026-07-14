//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'announcement_create_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AnnouncementCreateInput {
  /// Returns a new [AnnouncementCreateInput] instance.
  AnnouncementCreateInput({
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

    required this.reason,
  });

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: false, includeIfNull: false)
  final String? body;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementCreateInputStatusEnum.unknownDefaultOpenApi,
  )
  final AnnouncementCreateInputStatusEnum status;

  @JsonKey(
    name: r'presentation',
    required: true,
    includeIfNull: false,
    unknownEnumValue:
        AnnouncementCreateInputPresentationEnum.unknownDefaultOpenApi,
  )
  final AnnouncementCreateInputPresentationEnum presentation;

  @JsonKey(
    name: r'severity',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementCreateInputSeverityEnum.unknownDefaultOpenApi,
  )
  final AnnouncementCreateInputSeverityEnum severity;

  // minimum: -1000
  // maximum: 1000
  @JsonKey(name: r'priority', required: true, includeIfNull: false)
  final int priority;

  @JsonKey(
    name: r'audience',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementCreateInputAudienceEnum.unknownDefaultOpenApi,
  )
  final AnnouncementCreateInputAudienceEnum audience;

  @JsonKey(name: r'requiresAck', required: true, includeIfNull: false)
  final bool requiresAck;

  @JsonKey(name: r'startsAt', required: false, includeIfNull: false)
  final int? startsAt;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AnnouncementCreateInput &&
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
          other.reason == reason;

  @override
  int get hashCode =>
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
      reason.hashCode;

  factory AnnouncementCreateInput.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementCreateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementCreateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AnnouncementCreateInputStatusEnum {
  @JsonValue(r'draft')
  draft(r'draft'),
  @JsonValue(r'scheduled')
  scheduled(r'scheduled'),
  @JsonValue(r'published')
  published(r'published'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementCreateInputStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementCreateInputPresentationEnum {
  @JsonValue(r'card')
  card(r'card'),
  @JsonValue(r'banner')
  banner(r'banner'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementCreateInputPresentationEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementCreateInputSeverityEnum {
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

  const AnnouncementCreateInputSeverityEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AnnouncementCreateInputAudienceEnum {
  @JsonValue(r'all')
  all(r'all'),
  @JsonValue(r'authenticated')
  authenticated(r'authenticated'),
  @JsonValue(r'staff')
  staff(r'staff'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementCreateInputAudienceEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
