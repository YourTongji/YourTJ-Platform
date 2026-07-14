//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:json_annotation/json_annotation.dart';

part 'comment_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CommentUpdateInput {
  /// Returns a new [CommentUpdateInput] instance.
  CommentUpdateInput({
    this.expectedVersion = 1,

    required this.body,

    this.contentFormat,

    this.attachmentAssetIds,
  });

  /// Compare-and-swap version. Legacy omission is treated as version 1 and conflicts once content has changed.
  // minimum: 1
  @JsonKey(
    defaultValue: 1,
    name: r'expectedVersion',
    required: false,
    includeIfNull: false,
  )
  final int? expectedVersion;

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

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CommentUpdateInput &&
          other.expectedVersion == expectedVersion &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.attachmentAssetIds == attachmentAssetIds;

  @override
  int get hashCode =>
      expectedVersion.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      attachmentAssetIds.hashCode;

  factory CommentUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$CommentUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$CommentUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
