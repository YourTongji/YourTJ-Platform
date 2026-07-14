//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'profile_asset_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ProfileAssetInput {
  /// Returns a new [ProfileAssetInput] instance.
  ProfileAssetInput({required this.assetId});

  @JsonKey(name: r'assetId', required: true, includeIfNull: false)
  final String assetId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ProfileAssetInput && other.assetId == assetId;

  @override
  int get hashCode => assetId.hashCode;

  factory ProfileAssetInput.fromJson(Map<String, dynamic> json) =>
      _$ProfileAssetInputFromJson(json);

  Map<String, dynamic> toJson() => _$ProfileAssetInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
