// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'reviews_id_report_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReviewsIdReportPostRequest _$ReviewsIdReportPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ReviewsIdReportPostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason', 'captchaToken']);
  final val = ReviewsIdReportPostRequest(
    reason: $checkedConvert('reason', (v) => v as String),
    captchaToken: $checkedConvert('captchaToken', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$ReviewsIdReportPostRequestToJson(
  ReviewsIdReportPostRequest instance,
) => <String, dynamic>{
  'reason': instance.reason,
  'captchaToken': instance.captchaToken,
};
