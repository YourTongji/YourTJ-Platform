// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'me_patch_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MePatchRequest _$MePatchRequestFromJson(Map<String, dynamic> json) =>
    $checkedCreate('MePatchRequest', json, ($checkedConvert) {
      final val = MePatchRequest(
        handle: $checkedConvert('handle', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$MePatchRequestToJson(MePatchRequest instance) =>
    <String, dynamic>{'handle': ?instance.handle};
