//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'account_data_export.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AccountDataExport {
  /// Returns a new [AccountDataExport] instance.
  AccountDataExport({
    required this.schemaVersion,

    required this.generatedAt,

    required this.includedSections,

    required this.identity,

    required this.forum,

    required this.reviews,

    required this.governance,

    required this.credit,

    required this.activity,

    required this.platform,

    required this.media,
  });

  @JsonKey(name: r'schemaVersion', required: true, includeIfNull: false)
  final String schemaVersion;

  @JsonKey(name: r'generatedAt', required: true, includeIfNull: false)
  final int generatedAt;

  @JsonKey(name: r'includedSections', required: true, includeIfNull: false)
  final List<String> includedSections;

  @JsonKey(name: r'identity', required: true, includeIfNull: false)
  final Object identity;

  @JsonKey(name: r'forum', required: true, includeIfNull: false)
  final Object forum;

  @JsonKey(name: r'reviews', required: true, includeIfNull: false)
  final Object reviews;

  @JsonKey(name: r'governance', required: true, includeIfNull: false)
  final Object governance;

  @JsonKey(name: r'credit', required: true, includeIfNull: false)
  final Object credit;

  /// Owner-visible daily counts, check-ins, current trust projection, and redacted trust transition history.
  @JsonKey(name: r'activity', required: true, includeIfNull: false)
  final Object activity;

  @JsonKey(name: r'platform', required: true, includeIfNull: false)
  final Object platform;

  @JsonKey(name: r'media', required: true, includeIfNull: false)
  final List<Object> media;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AccountDataExport &&
          other.schemaVersion == schemaVersion &&
          other.generatedAt == generatedAt &&
          other.includedSections == includedSections &&
          other.identity == identity &&
          other.forum == forum &&
          other.reviews == reviews &&
          other.governance == governance &&
          other.credit == credit &&
          other.activity == activity &&
          other.platform == platform &&
          other.media == media;

  @override
  int get hashCode =>
      schemaVersion.hashCode +
      generatedAt.hashCode +
      includedSections.hashCode +
      identity.hashCode +
      forum.hashCode +
      reviews.hashCode +
      governance.hashCode +
      credit.hashCode +
      activity.hashCode +
      platform.hashCode +
      media.hashCode;

  factory AccountDataExport.fromJson(Map<String, dynamic> json) =>
      _$AccountDataExportFromJson(json);

  Map<String, dynamic> toJson() => _$AccountDataExportToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
