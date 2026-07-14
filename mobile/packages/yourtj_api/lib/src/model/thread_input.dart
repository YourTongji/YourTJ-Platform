//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/poll_input.dart';
import 'package:json_annotation/json_annotation.dart';

part 'thread_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadInput {
  /// Returns a new [ThreadInput] instance.
  ThreadInput({
    required this.boardId,

    required this.title,

    this.body,

    this.contentFormat,

    this.tags,

    this.attachmentAssetIds,

    this.poll,
  });

  @JsonKey(name: r'boardId', required: true, includeIfNull: false)
  final String boardId;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: false, includeIfNull: false)
  final String? body;

  @JsonKey(
    name: r'contentFormat',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat? contentFormat;

  @JsonKey(name: r'tags', required: false, includeIfNull: false)
  final Set<String>? tags;

  /// Ordered asset ids that must exactly equal the markdown_v1 yourtj-asset image destinations. Must be empty for plain_v1.
  @JsonKey(name: r'attachmentAssetIds', required: false, includeIfNull: false)
  final Set<String>? attachmentAssetIds;

  @JsonKey(name: r'poll', required: false, includeIfNull: false)
  final PollInput? poll;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadInput &&
          other.boardId == boardId &&
          other.title == title &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.tags == tags &&
          other.attachmentAssetIds == attachmentAssetIds &&
          other.poll == poll;

  @override
  int get hashCode =>
      boardId.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      contentFormat.hashCode +
      tags.hashCode +
      attachmentAssetIds.hashCode +
      poll.hashCode;

  factory ThreadInput.fromJson(Map<String, dynamic> json) =>
      _$ThreadInputFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
