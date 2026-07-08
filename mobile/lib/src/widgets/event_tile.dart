import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import '../models/models.dart';

class EventTile extends StatelessWidget {
  final MotionEvent event;
  final String cameraName;

  const EventTile({
    super.key,
    required this.event,
    required this.cameraName,
  });

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return ListTile(
      dense: true,
      leading: Container(
        width:  36,
        height: 36,
        decoration: BoxDecoration(
          color: Colors.orange.withOpacity(0.15),
          borderRadius: BorderRadius.circular(8),
        ),
        child: const Icon(Icons.directions_run,
            color: Colors.orange, size: 20),
      ),
      title: Text(
        cameraName,
        style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
      ),
      subtitle: Text(
        '${_dateLabel(event.occurredAt)}  ·  ${event.source}'
        '${event.score != null ? '  ·  ${(event.score! * 100).toStringAsFixed(0)}%' : ''}',
        style: TextStyle(
          fontSize: 11,
          color: cs.onSurface.withOpacity(0.5),
        ),
      ),
      trailing: Text(
        event.timeLabel,
        style: TextStyle(
          fontSize: 11,
          color: cs.onSurface.withOpacity(0.4),
        ),
      ),
    );
  }

  static String _dateLabel(DateTime dt) {
    final now   = DateTime.now();
    final today = DateTime(now.year, now.month, now.day);
    final day   = DateTime(dt.year, dt.month, dt.day);
    if (day == today) return 'Today';
    if (today.difference(day).inDays == 1) return 'Yesterday';
    return DateFormat('MMM d').format(dt);
  }
}
