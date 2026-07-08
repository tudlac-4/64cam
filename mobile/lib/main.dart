import 'package:flutter/material.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'app.dart';

final flutterLocalNotificationsPlugin = FlutterLocalNotificationsPlugin();

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialise local-notification channels (Android 8+ requires a channel).
  const android = AndroidInitializationSettings('@mipmap/ic_launcher');
  const darwin = DarwinInitializationSettings(
    requestAlertPermission: false, // requested on first motion event
    requestBadgePermission: false,
    requestSoundPermission: false,
  );
  await flutterLocalNotificationsPlugin.initialize(
    const InitializationSettings(android: android, iOS: darwin),
  );

  runApp(const ProviderScope(child: Cam64App()));
}
