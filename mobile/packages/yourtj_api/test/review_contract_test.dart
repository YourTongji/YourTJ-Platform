import 'package:json_annotation/json_annotation.dart';
import 'package:test/test.dart';
import 'package:yourtj_api/yourtj_api.dart';

void main() {
  test('decodes server-authoritative review viewer state', () {
    final Review review = Review.fromJson(<String, dynamic>{
      'id': 'review-1',
      'viewerLiked': true,
      'canEdit': false,
      'canReport': true,
    });

    expect(review.viewerLiked, isTrue);
    expect(review.canEdit, isFalse);
    expect(review.canReport, isTrue);
  });

  test('rejects review payloads that omit viewer state', () {
    expect(
      () => Review.fromJson(<String, dynamic>{'id': 'review-1'}),
      throwsA(isA<CheckedFromJsonException>()),
    );
  });
}
