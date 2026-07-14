import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/home/domain/promotion_presentation.dart';

void main() {
  MediaDelivery delivery({
    String url = 'https://cdn.yourtj.de/delivery/avatar',
    int expiresAt = 200,
    int width = 800,
    int height = 450,
  }) {
    return MediaDelivery(
      assetId: 'asset-1',
      variant: MediaDeliveryVariant.display1280,
      url: url,
      expiresAt: expiresAt,
      mime: MediaDeliveryMimeEnum.imageSlashWebp,
      width: width,
      height: height,
    );
  }

  test('accepts only fresh HTTPS promotion deliveries', () {
    expect(
      PromotionPresentation.freshImageUri(delivery(), now: 100),
      Uri.parse('https://cdn.yourtj.de/delivery/avatar'),
    );
    expect(
      PromotionPresentation.freshImageUri(delivery(expiresAt: 130), now: 100),
      isNull,
    );
    expect(
      PromotionPresentation.freshImageUri(
        delivery(url: 'http://cdn.yourtj.de/delivery/avatar'),
        now: 100,
      ),
      isNull,
    );
    expect(
      PromotionPresentation.freshImageUri(
        delivery(url: 'https://user@cdn.yourtj.de/delivery/avatar'),
        now: 100,
      ),
      isNull,
    );
    expect(
      PromotionPresentation.freshImageUri(delivery(width: 0), now: 100),
      isNull,
    );
  });
}
