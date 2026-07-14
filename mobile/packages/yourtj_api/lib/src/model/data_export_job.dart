//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/data_export_status.dart';
import 'package:json_annotation/json_annotation.dart';

part 'data_export_job.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DataExportJob {
  /// Returns a new [DataExportJob] instance.
  DataExportJob({
    required this.id,

    required this.status,

    required this.createdAt,

    required this.updatedAt,

    required this.expiresAt,

    required this.errorCode,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DataExportStatus.unknownDefaultOpenApi,
  )
  final DataExportStatus status;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'updatedAt', required: true, includeIfNull: false)
  final int updatedAt;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @JsonKey(name: r'errorCode', required: true, includeIfNull: true)
  final String? errorCode;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DataExportJob &&
          other.id == id &&
          other.status == status &&
          other.createdAt == createdAt &&
          other.updatedAt == updatedAt &&
          other.expiresAt == expiresAt &&
          other.errorCode == errorCode;

  @override
  int get hashCode =>
      id.hashCode +
      status.hashCode +
      createdAt.hashCode +
      updatedAt.hashCode +
      expiresAt.hashCode +
      (errorCode == null ? 0 : errorCode.hashCode);

  factory DataExportJob.fromJson(Map<String, dynamic> json) =>
      _$DataExportJobFromJson(json);

  Map<String, dynamic> toJson() => _$DataExportJobToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
