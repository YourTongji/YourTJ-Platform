//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'announcement_receipt_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AnnouncementReceiptInput {
  /// Returns a new [AnnouncementReceiptInput] instance.
  AnnouncementReceiptInput({required this.revision, required this.action});

  // minimum: 1
  @JsonKey(name: r'revision', required: true, includeIfNull: false)
  final int revision;

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AnnouncementReceiptInputActionEnum.unknownDefaultOpenApi,
  )
  final AnnouncementReceiptInputActionEnum action;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AnnouncementReceiptInput &&
          other.revision == revision &&
          other.action == action;

  @override
  int get hashCode => revision.hashCode + action.hashCode;

  factory AnnouncementReceiptInput.fromJson(Map<String, dynamic> json) =>
      _$AnnouncementReceiptInputFromJson(json);

  Map<String, dynamic> toJson() => _$AnnouncementReceiptInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AnnouncementReceiptInputActionEnum {
  @JsonValue(r'seen')
  seen(r'seen'),
  @JsonValue(r'dismiss')
  dismiss(r'dismiss'),
  @JsonValue(r'acknowledge')
  acknowledge(r'acknowledge'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AnnouncementReceiptInputActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
