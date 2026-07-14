//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'forum_attachment.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ForumAttachment {
  /// Returns a new [ForumAttachment] instance.
  ForumAttachment({
    required this.assetId,

    required this.reference,

    required this.position,

    required this.alt,

    required this.url,

    required this.expiresAt,

    required this.width,

    required this.height,
  });

  @JsonKey(name: r'assetId', required: true, includeIfNull: false)
  final String assetId;

  @JsonKey(name: r'reference', required: true, includeIfNull: false)
  final String reference;

  // minimum: 0
  // maximum: 7
  @JsonKey(name: r'position', required: true, includeIfNull: false)
  final int position;

  @JsonKey(name: r'alt', required: true, includeIfNull: false)
  final String alt;

  /// Authorization-derived clean image URL; never persisted in Forum content.
  @JsonKey(name: r'url', required: true, includeIfNull: false)
  final String url;

  /// Unix seconds; refetch the owning Forum resource before or after expiry rather than calling an owner-only Media route.
  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  // minimum: 1
  // maximum: 1280
  @JsonKey(name: r'width', required: true, includeIfNull: true)
  final int? width;

  // minimum: 1
  // maximum: 1280
  @JsonKey(name: r'height', required: true, includeIfNull: true)
  final int? height;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ForumAttachment &&
          other.assetId == assetId &&
          other.reference == reference &&
          other.position == position &&
          other.alt == alt &&
          other.url == url &&
          other.expiresAt == expiresAt &&
          other.width == width &&
          other.height == height;

  @override
  int get hashCode =>
      assetId.hashCode +
      reference.hashCode +
      position.hashCode +
      alt.hashCode +
      url.hashCode +
      expiresAt.hashCode +
      (width == null ? 0 : width.hashCode) +
      (height == null ? 0 : height.hashCode);

  factory ForumAttachment.fromJson(Map<String, dynamic> json) =>
      _$ForumAttachmentFromJson(json);

  Map<String, dynamic> toJson() => _$ForumAttachmentToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
