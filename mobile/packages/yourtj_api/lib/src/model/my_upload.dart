//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_delivery_state.dart';
import 'package:yourtj_api/src/model/media_usage.dart';
import 'package:json_annotation/json_annotation.dart';

part 'my_upload.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MyUpload {
  /// Returns a new [MyUpload] instance.
  MyUpload({
    required this.id,

    required this.kind,

    required this.usage,

    required this.bytes,

    required this.mime,

    required this.status,

    required this.deliveryState,

    required this.imageWidth,

    required this.imageHeight,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'kind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MyUploadKindEnum.unknownDefaultOpenApi,
  )
  final MyUploadKindEnum kind;

  @JsonKey(
    name: r'usage',
    required: true,
    includeIfNull: true,
    unknownEnumValue: MediaUsage.unknownDefaultOpenApi,
  )
  final MediaUsage? usage;

  /// Zero only after an object was physically deleted and its metadata redacted.
  // minimum: 0
  @JsonKey(name: r'bytes', required: true, includeIfNull: false)
  final int bytes;

  @JsonKey(name: r'mime', required: true, includeIfNull: false)
  final String mime;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MyUploadStatusEnum.unknownDefaultOpenApi,
  )
  final MyUploadStatusEnum status;

  @JsonKey(
    name: r'deliveryState',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeliveryState.unknownDefaultOpenApi,
  )
  final MediaDeliveryState deliveryState;

  // minimum: 1
  // maximum: 20000
  @JsonKey(name: r'imageWidth', required: true, includeIfNull: true)
  final int? imageWidth;

  // minimum: 1
  // maximum: 20000
  @JsonKey(name: r'imageHeight', required: true, includeIfNull: true)
  final int? imageHeight;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MyUpload &&
          other.id == id &&
          other.kind == kind &&
          other.usage == usage &&
          other.bytes == bytes &&
          other.mime == mime &&
          other.status == status &&
          other.deliveryState == deliveryState &&
          other.imageWidth == imageWidth &&
          other.imageHeight == imageHeight &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      kind.hashCode +
      (usage == null ? 0 : usage.hashCode) +
      bytes.hashCode +
      mime.hashCode +
      status.hashCode +
      deliveryState.hashCode +
      (imageWidth == null ? 0 : imageWidth.hashCode) +
      (imageHeight == null ? 0 : imageHeight.hashCode) +
      createdAt.hashCode;

  factory MyUpload.fromJson(Map<String, dynamic> json) =>
      _$MyUploadFromJson(json);

  Map<String, dynamic> toJson() => _$MyUploadToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum MyUploadKindEnum {
  @JsonValue(r'image')
  image(r'image'),
  @JsonValue(r'file')
  file(r'file'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MyUploadKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum MyUploadStatusEnum {
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'clean')
  clean(r'clean'),
  @JsonValue(r'quarantined')
  quarantined(r'quarantined'),
  @JsonValue(r'blocked')
  blocked(r'blocked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MyUploadStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
