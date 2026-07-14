//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'onebox_result.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class OneboxResult {
  /// Returns a new [OneboxResult] instance.
  OneboxResult({
    required this.type,

    required this.url,

    required this.title,

    required this.description,

    required this.imageUrl,

    required this.siteName,
  });

  /// plain means the host is not allowlisted; card means metadata was fetched or read from cache.
  @JsonKey(
    name: r'type',
    required: true,
    includeIfNull: false,
    unknownEnumValue: OneboxResultTypeEnum.unknownDefaultOpenApi,
  )
  final OneboxResultTypeEnum type;

  @JsonKey(name: r'url', required: true, includeIfNull: false)
  final String url;

  @JsonKey(name: r'title', required: true, includeIfNull: true)
  final String? title;

  @JsonKey(name: r'description', required: true, includeIfNull: true)
  final String? description;

  /// Reserved for a future platform-proxied image; remote preview images are currently returned as null.
  @JsonKey(name: r'imageUrl', required: true, includeIfNull: true)
  final String? imageUrl;

  @JsonKey(name: r'siteName', required: true, includeIfNull: true)
  final String? siteName;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is OneboxResult &&
          other.type == type &&
          other.url == url &&
          other.title == title &&
          other.description == description &&
          other.imageUrl == imageUrl &&
          other.siteName == siteName;

  @override
  int get hashCode =>
      type.hashCode +
      url.hashCode +
      (title == null ? 0 : title.hashCode) +
      (description == null ? 0 : description.hashCode) +
      (imageUrl == null ? 0 : imageUrl.hashCode) +
      (siteName == null ? 0 : siteName.hashCode);

  factory OneboxResult.fromJson(Map<String, dynamic> json) =>
      _$OneboxResultFromJson(json);

  Map<String, dynamic> toJson() => _$OneboxResultToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

/// plain means the host is not allowlisted; card means metadata was fetched or read from cache.
enum OneboxResultTypeEnum {
  /// plain means the host is not allowlisted; card means metadata was fetched or read from cache.
  @JsonValue(r'plain')
  plain(r'plain'),

  /// plain means the host is not allowlisted; card means metadata was fetched or read from cache.
  @JsonValue(r'card')
  card(r'card'),

  /// plain means the host is not allowlisted; card means metadata was fetched or read from cache.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const OneboxResultTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
