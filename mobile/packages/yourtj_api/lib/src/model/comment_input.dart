//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:json_annotation/json_annotation.dart';

part 'comment_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CommentInput {
  /// Returns a new [CommentInput] instance.
  CommentInput({
    this.parentId,

    required this.body,

    this.contentFormat,

    this.attachmentAssetIds,

    this.quotedCommentId,
  });

  @JsonKey(name: r'parentId', required: false, includeIfNull: false)
  final String? parentId;

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @JsonKey(
    name: r'contentFormat',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat? contentFormat;

  /// Ordered asset ids that must exactly match markdown_v1 yourtj-asset image destinations.
  @JsonKey(name: r'attachmentAssetIds', required: false, includeIfNull: false)
  final Set<String>? attachmentAssetIds;

  @JsonKey(name: r'quotedCommentId', required: false, includeIfNull: false)
  final String? quotedCommentId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CommentInput &&
          other.parentId == parentId &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.attachmentAssetIds == attachmentAssetIds &&
          other.quotedCommentId == quotedCommentId;

  @override
  int get hashCode =>
      parentId.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      attachmentAssetIds.hashCode +
      quotedCommentId.hashCode;

  factory CommentInput.fromJson(Map<String, dynamic> json) =>
      _$CommentInputFromJson(json);

  Map<String, dynamic> toJson() => _$CommentInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
