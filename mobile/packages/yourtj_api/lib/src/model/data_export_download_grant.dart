//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'data_export_download_grant.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DataExportDownloadGrant {
  /// Returns a new [DataExportDownloadGrant] instance.
  DataExportDownloadGrant({required this.token, required this.expiresAt});

  @JsonKey(name: r'token', required: true, includeIfNull: false)
  final String token;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DataExportDownloadGrant &&
          other.token == token &&
          other.expiresAt == expiresAt;

  @override
  int get hashCode => token.hashCode + expiresAt.hashCode;

  factory DataExportDownloadGrant.fromJson(Map<String, dynamic> json) =>
      _$DataExportDownloadGrantFromJson(json);

  Map<String, dynamic> toJson() => _$DataExportDownloadGrantToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
