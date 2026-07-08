import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/models.dart';
import '../providers/providers.dart';
import '../widgets/event_tile.dart';

class EventsScreen extends ConsumerWidget {
  const EventsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dash    = ref.watch(dashboardProvider);
    final events  = dash.events;
    final cameras = ref.watch(camerasProvider).valueOrNull;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Motion Events'),
        actions: [
          if (events.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.delete_sweep_outlined, size: 20),
              onPressed: () => ref
                  .read(dashboardProvider.notifier)
                  .clearEvents(),
              tooltip: 'Clear events',
            ),
        ],
      ),
      body: events.isEmpty
          ? const Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.notifications_off_outlined,
                      size: 48, color: Colors.white24),
                  SizedBox(height: 16),
                  Text('No motion events yet',
                      style: TextStyle(color: Colors.white38)),
                ],
              ),
            )
          : ListView.separated(
              padding: const EdgeInsets.symmetric(vertical: 8),
              itemCount: events.length,
              separatorBuilder: (_, __) => const Divider(height: 1),
              itemBuilder: (context, i) {
                final evt = events[i];
                final cameraName = cameras
                    ?.where((c) => c.id == evt.cameraId)
                    .map((c) => c.name)
                    .firstOrNull;
                return EventTile(
                  event: evt,
                  cameraName: cameraName ?? evt.cameraId.substring(0, 8),
                );
              },
            ),
    );
  }
}
