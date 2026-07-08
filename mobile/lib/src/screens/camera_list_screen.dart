import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../providers/providers.dart';
import '../widgets/camera_card.dart';

class CameraListScreen extends ConsumerWidget {
  const CameraListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final camerasAsync = ref.watch(camerasProvider);
    final dash         = ref.watch(dashboardProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('64cam'),
        actions: [
          // Connection status indicator
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Icon(
              dash.connected ? Icons.cloud_done : Icons.cloud_off,
              size: 20,
              color: dash.connected
                  ? Colors.green.shade400
                  : Theme.of(context).colorScheme.onSurface.withOpacity(0.3),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.logout, size: 20),
            onPressed: () async {
              await ref.read(authProvider.notifier).logout();
              if (context.mounted) context.go('/login');
            },
            tooltip: 'Sign out',
          ),
        ],
      ),
      body: camerasAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error:   (e, _) => Center(child: Text('$e')),
        data:    (cameras) {
          if (cameras.isEmpty) {
            return const Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.videocam_off, size: 48, color: Colors.white24),
                  SizedBox(height: 16),
                  Text('No cameras configured',
                    style: TextStyle(color: Colors.white38)),
                ],
              ),
            );
          }
          return RefreshIndicator(
            onRefresh: () => ref.refresh(camerasProvider.future),
            child: GridView.builder(
              padding: const EdgeInsets.all(12),
              gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                crossAxisCount:    2,
                mainAxisSpacing:   8,
                crossAxisSpacing:  8,
                childAspectRatio:  4 / 3,
              ),
              itemCount: cameras.length,
              itemBuilder: (context, i) {
                final cam    = cameras[i];
                final status = dash.cameraStatuses[cam.id];
                return CameraCard(
                  camera: cam,
                  status: status,
                  onTap:     () => context.go('/home/live/${cam.id}'),
                  onPlayback: () => context.go('/home/playback/${cam.id}'),
                );
              },
            ),
          );
        },
      ),
    );
  }
}
