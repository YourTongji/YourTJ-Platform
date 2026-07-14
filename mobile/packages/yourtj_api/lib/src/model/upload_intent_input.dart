//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_usage.dart';
import 'package:json_annotation/json_annotation.dart';

part 'upload_intent_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UploadIntentInput {
  /// Returns a new [UploadIntentInput] instance.
  UploadIntentInput({
    required this.kind,

    required this.contentType,

    this.usage,
  });

  @JsonKey(
    name: r'kind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UploadIntentInputKindEnum.unknownDefaultOpenApi,
  )
  final UploadIntentInputKindEnum kind;

  @JsonKey(name: r'contentType', required: true, includeIfNull: false)
  final String contentType;

  /// Controlled usages require kind=image. Omit for unbound generic uploads.
  @JsonKey(
    name: r'usage',
    required: false,
    includeIfNull: false,
    unknownEnumValue: MediaUsage.unknownDefaultOpenApi,
  )
  final MediaUsage? usage;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UploadIntentInput &&
          other.kind == kind &&
          other.contentType == contentType &&
          other.usage == usage;

  @override
  int get hashCode => kind.hashCode + contentType.hashCode + usage.hashCode;

  factory UploadIntentInput.fromJson(Map<String, dynamic> json) =>
      _$UploadIntentInputFromJson(json);

  Map<String, dynamic> toJson() => _$UploadIntentInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UploadIntentInputKindEnum {
  @JsonValue(r'image')
  image(r'image'),
  @JsonValue(r'file')
  file(r'file'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadIntentInputKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
