import 'dart:async';
import 'dart:convert';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../../main.dart';
import '../models/models.dart';

const _maxEvents = 100;
const _reconnectDelay = Duration(seconds: 5);

class DashboardState {
  final Map<String, CameraRtStatus> cameraStatuses;
  final List<MotionEvent> events;
  final bool connected;

  const DashboardState({
    this.cameraStatuses = const {},
    this.events         = const [],
    this.connected      = false,
  });

  DashboardState copyWith({
    Map<String, CameraRtStatus>? cameraStatuses,
    List<MotionEvent>?           events,
    bool?                        connected,
  }) =>
      DashboardState(
        cameraStatuses: cameraStatuses ?? this.cameraStatuses,
        events:         events         ?? this.events,
        connected:      connected      ?? this.connected,
      );
}

class DashboardNotifier extends Notifier<DashboardState> {
  WebSocketChannel? _channel;
  StreamSubscription<dynamic>? _sub;
  Timer? _reconnectTimer;
  String? _url;

  @override
  DashboardState build() => const DashboardState();

  void connect(String wsUrl) {
    _url = wsUrl;
    _doConnect();
  }

  void _doConnect() {
    _sub?.cancel();
    try {
      _channel = WebSocketChannel.connect(Uri.parse(_url!));
      state    = state.copyWith(connected: true);
      _sub     = _channel!.stream.listen(
        _onMessage,
        onError: (_) => _scheduleReconnect(),
        onDone:  _scheduleReconnect,
      );
    } catch (_) {
      _scheduleReconnect();
    }
  }

  void _scheduleReconnect() {
    state = state.copyWith(connected: false);
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(_reconnectDelay, _doConnect);
  }

  void _onMessage(dynamic raw) {
    try {
      final json = jsonDecode(raw as String) as Map<String, dynamic>;
      final type = json['type'] as String?;

      switch (type) {
        case 'camera_status_update':
          final updates = (json['cameras'] as List<dynamic>)
              .cast<Map<String, dynamic>>()
              .map((c) => CameraRtStatus(
                    cameraId:  c['camera_id'] as String,
                    connected: c['connected'] as bool? ?? false,
                    readers:   c['readers'] as int? ?? 0,
                  ));
          final next = Map<String, CameraRtStatus>.from(state.cameraStatuses);
          for (final u in updates) {
            next[u.cameraId] = u;
          }
          state = state.copyWith(cameraStatuses: next);

        case 'motion_event':
          final evt = MotionEvent.fromJson(json);
          final next = [evt, ...state.events].take(_maxEvents).toList();
          state = state.copyWith(events: next);
          _notify(evt);
      }
    } catch (_) {
      // ignore malformed frames
    }
  }

  void _notify(MotionEvent evt) {
    // Show an in-app notification banner.
    // For background push notifications, integrate FCM/APNs separately.
    const androidDetails = AndroidNotificationDetails(
      'motion_events',
      'Motion Events',
      channelDescription: 'Alerts when motion is detected by a camera',
      importance: Importance.high,
      priority:   Priority.high,
      playSound:  true,
    );
    const iosDetails = DarwinNotificationDetails(
      presentAlert: true,
      presentSound: true,
    );
    flutterLocalNotificationsPlugin.show(
      evt.occurredAt.millisecondsSinceEpoch ~/ 1000,
      'Motion detected',
      'Camera • ${evt.timeLabel}',
      const NotificationDetails(android: androidDetails, iOS: iosDetails),
    );
  }

  void clearEvents() => state = state.copyWith(events: const []);

  @override
  void dispose() {
    _reconnectTimer?.cancel();
    _sub?.cancel();
    _channel?.sink.close();
    super.dispose();
  }
}
