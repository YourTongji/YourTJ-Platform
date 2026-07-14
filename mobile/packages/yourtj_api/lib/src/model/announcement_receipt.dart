//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'announcement_receipt.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AnnouncementReceipt {
  /// Returns a new [AnnouncementReceipt] instance.
  AnnouncementReceipt({
    required this.revision,

    this.firstSeenAt,

    this.dismissedAt,

    this.acknowledgedAt,
  });

  // minimum: 1
  @JsonKey(name: r'revision', required: true, includeIfNull: false)
  final int revision;

  @JsonKey(name: r'firstSeenAt', required: false, includeIfNull: false)
  final int? firstSeenAt;

  @JsonKey(name: r'dismissedAt', required: false, includeIfNull: false)
  final int? dismissedAt;

  @JsonKey(name: r'acknowledgedAt', required: false, includeIfNull: false)
  final int? acknowledgedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AnnouncementReceipt &&
          other.revision == revision &&
          other.firstSeenAt == firstSeenAt &&
          other.dismissedAt == dismissedAt &&
          other.acknowledgedAt == acknowledgedAt;

  @override
  int get hashCode =>
      revision.hashCode +
      (firstSeenAt == null ? 0 : firstSeenAt.hashCode) +
      (dismissedAt == null ? 0 : dismissedAt.hashCode) +
      (acknowledgedAt == null ? 0 : acknowledgedAt.hashCode);

  factory AnnouncementReceipt.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementReceiptFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementReceiptToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
