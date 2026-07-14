//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'media_provider_inventory_status.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaProviderInventoryStatus {
  /// Returns a new [MediaProviderInventoryStatus] instance.
  MediaProviderInventoryStatus({
    required this.state,

    required this.ingestCandidateCount,

    required this.deliveryCandidateCount,
  });

  @JsonKey(
    name: r'state',
    required: true,
    includeIfNull: false,
    unknownEnumValue:
        MediaProviderInventoryStatusStateEnum.unknownDefaultOpenApi,
  )
  final MediaProviderInventoryStatusStateEnum state;

  // minimum: 0
  @JsonKey(name: r'ingestCandidateCount', required: true, includeIfNull: false)
  final int ingestCandidateCount;

  // minimum: 0
  @JsonKey(
    name: r'deliveryCandidateCount',
    required: true,
    includeIfNull: false,
  )
  final int deliveryCandidateCount;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaProviderInventoryStatus &&
          other.state == state &&
          other.ingestCandidateCount == ingestCandidateCount &&
          other.deliveryCandidateCount == deliveryCandidateCount;

  @override
  int get hashCode =>
      state.hashCode +
      ingestCandidateCount.hashCode +
      deliveryCandidateCount.hashCode;

  factory MediaProviderInventoryStatus.fromJson(Map<String, dynamic> json) =>
      _$MediaProviderInventoryStatusFromJson(json);

  Map<String, dynamic> toJson() => _$MediaProviderInventoryStatusToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum MediaProviderInventoryStatusStateEnum {
  @JsonValue(r'manual_inventory_required')
  manualInventoryRequired(r'manual_inventory_required'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaProviderInventoryStatusStateEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
