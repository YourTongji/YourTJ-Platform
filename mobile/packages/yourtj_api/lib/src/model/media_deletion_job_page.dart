//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_deletion_job.dart';
import 'package:json_annotation/json_annotation.dart';

part 'media_deletion_job_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaDeletionJobPage {
  /// Returns a new [MediaDeletionJobPage] instance.
  MediaDeletionJobPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<MediaDeletionJob> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaDeletionJobPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory MediaDeletionJobPage.fromJson(Map<String, dynamic> json) =>
      _$MediaDeletionJobPageFromJson(json);

  Map<String, dynamic> toJson() => _$MediaDeletionJobPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
