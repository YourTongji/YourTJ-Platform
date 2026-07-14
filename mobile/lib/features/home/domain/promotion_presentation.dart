import 'package:yourtj_api/yourtj_api.dart';

abstract final class PromotionPresentation {
  static const int deliveryRefreshSkewSeconds = 30;

  static Uri? freshImageUri(MediaDelivery delivery, {required int now}) {
    final Uri? uri = Uri.tryParse(delivery.url.trim());
    if (delivery.expiresAt <= now + deliveryRefreshSkewSeconds ||
        delivery.width <= 0 ||
        delivery.height <= 0 ||
        uri == null ||
        uri.scheme != 'https' ||
        !uri.hasAuthority ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasFragment) {
      return null;
    }
    return uri;
  }
}
