//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'sanction.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Sanction {
  /// Returns a new [Sanction] instance.
  Sanction({
    this.id,

    this.accountId,

    this.kind,

    this.reason,

    this.issuedBy,

    this.startsAt,

    this.endsAt,

    this.revokedAt,

    this.createdAt,
  });

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'accountId', required: false, includeIfNull: false)
  final String? accountId;

  @JsonKey(
    name: r'kind',
    required: false,
    includeIfNull: false,
    unknownEnumValue: SanctionKindEnum.unknownDefaultOpenApi,
  )
  final SanctionKindEnum? kind;

  @JsonKey(name: r'reason', required: false, includeIfNull: false)
  final String? reason;

  @JsonKey(name: r'issuedBy', required: false, includeIfNull: false)
  final String? issuedBy;

  @JsonKey(name: r'startsAt', required: false, includeIfNull: false)
  final int? startsAt;

  @JsonKey(name: r'endsAt', required: false, includeIfNull: false)
  final int? endsAt;

  @JsonKey(name: r'revokedAt', required: false, includeIfNull: false)
  final int? revokedAt;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Sanction &&
          other.id == id &&
          other.accountId == accountId &&
          other.kind == kind &&
          other.reason == reason &&
          other.issuedBy == issuedBy &&
          other.startsAt == startsAt &&
          other.endsAt == endsAt &&
          other.revokedAt == revokedAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      accountId.hashCode +
      kind.hashCode +
      reason.hashCode +
      (issuedBy == null ? 0 : issuedBy.hashCode) +
      startsAt.hashCode +
      (endsAt == null ? 0 : endsAt.hashCode) +
      (revokedAt == null ? 0 : revokedAt.hashCode) +
      createdAt.hashCode;

  factory Sanction.fromJson(Map<String, dynamic> json) =>
      _$SanctionFromJson(json);

  Map<String, dynamic> toJson() => _$SanctionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum SanctionKindEnum {
  @JsonValue(r'silence')
  silence(r'silence'),
  @JsonValue(r'suspend')
  suspend(r'suspend'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SanctionKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
