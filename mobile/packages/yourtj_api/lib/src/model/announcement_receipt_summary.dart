//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'announcement_receipt_summary.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AnnouncementReceiptSummary {
  /// Returns a new [AnnouncementReceiptSummary] instance.
  AnnouncementReceiptSummary({
    required this.seenCount,

    required this.dismissedCount,

    required this.acknowledgedCount,
  });

  // minimum: 0
  @JsonKey(name: r'seenCount', required: true, includeIfNull: false)
  final int seenCount;

  // minimum: 0
  @JsonKey(name: r'dismissedCount', required: true, includeIfNull: false)
  final int dismissedCount;

  // minimum: 0
  @JsonKey(name: r'acknowledgedCount', required: true, includeIfNull: false)
  final int acknowledgedCount;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AnnouncementReceiptSummary &&
          other.seenCount == seenCount &&
          other.dismissedCount == dismissedCount &&
          other.acknowledgedCount == acknowledgedCount;

  @override
  int get hashCode =>
      seenCount.hashCode + dismissedCount.hashCode + acknowledgedCount.hashCode;

  factory AnnouncementReceiptSummary.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementReceiptSummaryFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementReceiptSummaryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
