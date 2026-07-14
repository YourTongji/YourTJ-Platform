import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app/app.dart';
import 'app/app_services.dart';
import 'app/router.dart';
import 'core/widgets/platform_avatar.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    final AppServices services = await AppServices.create();
    final router = createAppRouter(session: services.session);
    runApp(
      ProviderScope(
        overrides: [appServicesProvider.overrideWithValue(services)],
        child: PlatformMediaScope(
          environment: services.environment,
          child: YourTjApp(router: router),
        ),
      ),
    );
  } on Object {
    runApp(const _BootstrapFailureApp());
  }
}

class _BootstrapFailureApp extends StatelessWidget {
  const _BootstrapFailureApp();

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      debugShowCheckedModeBanner: false,
      home: Scaffold(
        body: SafeArea(
          child: Center(
            child: Padding(
              padding: EdgeInsets.all(24),
              child: Text('客户端配置无效，已停止连接。请更新应用或联系管理员。'),
            ),
          ),
        ),
      ),
    );
  }
}
