//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/dm_message.dart';
import 'package:json_annotation/json_annotation.dart';

part 'dm_message_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmMessagePage {
  /// Returns a new [DmMessagePage] instance.
  DmMessagePage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<DmMessage> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmMessagePage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory DmMessagePage.fromJson(Map<String, dynamic> json) =>
      _$DmMessagePageFromJson(json);

  Map<String, dynamic> toJson() => _$DmMessagePageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
