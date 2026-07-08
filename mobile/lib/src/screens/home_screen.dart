import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/providers.dart';
import 'camera_list_screen.dart';
import 'events_screen.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  int _tab = 0;

  @override
  Widget build(BuildContext context) {
    final dash        = ref.watch(dashboardProvider);
    final eventCount  = dash.events.length;

    return Scaffold(
      body: IndexedStack(
        index: _tab,
        children: const [
          CameraListScreen(),
          EventsScreen(),
        ],
      ),
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _tab,
        onTap: (i) => setState(() => _tab = i),
        items: [
          const BottomNavigationBarItem(
            icon: Icon(Icons.videocam_outlined),
            activeIcon: Icon(Icons.videocam),
            label: 'Cameras',
          ),
          BottomNavigationBarItem(
            icon: Badge(
              isLabelVisible: eventCount > 0 && _tab != 1,
              label: Text('$eventCount'),
              child: const Icon(Icons.notifications_outlined),
            ),
            activeIcon: const Icon(Icons.notifications),
            label: 'Events',
          ),
        ],
      ),
    );
  }
}
