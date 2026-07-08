import 'package:flutter/material.dart';

import '../models/models.dart';

class CameraCard extends StatelessWidget {
  final Camera camera;
  final CameraRtStatus? status;
  final VoidCallback onTap;
  final VoidCallback onPlayback;

  const CameraCard({
    super.key,
    required this.camera,
    this.status,
    required this.onTap,
    required this.onPlayback,
  });

  @override
  Widget build(BuildContext context) {
    final isOnline = status?.connected ?? false;
    final cs       = Theme.of(context).colorScheme;

    return GestureDetector(
      onTap: onTap,
      child: Container(
        decoration: BoxDecoration(
          color: Theme.of(context).cardColor,
          borderRadius: BorderRadius.circular(8),
          border: Border.all(
            color: isOnline
                ? cs.primary.withOpacity(0.25)
                : Colors.white.withOpacity(0.06),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Thumbnail area (placeholder — no snapshot endpoint yet)
            Expanded(
              child: ClipRRect(
                borderRadius:
                    const BorderRadius.vertical(top: Radius.circular(7)),
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    Container(color: const Color(0xFF0c0c0c)),
                    if (isOnline)
                      const Icon(Icons.videocam,
                          color: Colors.white12, size: 40)
                    else
                      const Icon(Icons.videocam_off,
                          color: Colors.white10, size: 36),
                    // Online indicator dot
                    Positioned(
                      top: 6,
                      left: 6,
                      child: Container(
                        width:  8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: isOnline ? Colors.green : Colors.grey.shade700,
                          shape: BoxShape.circle,
                        ),
                      ),
                    ),
                    // Viewer count
                    if ((status?.readers ?? 0) > 0)
                      Positioned(
                        top: 4,
                        right: 4,
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 5, vertical: 2),
                          decoration: BoxDecoration(
                            color: Colors.black54,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(Icons.remove_red_eye,
                                  size: 10, color: Colors.white70),
                              const SizedBox(width: 3),
                              Text(
                                '${status!.readers}',
                                style: const TextStyle(
                                    fontSize: 10, color: Colors.white70),
                              ),
                            ],
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),

            // Footer
            Padding(
              padding:
                  const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      camera.name,
                      style: const TextStyle(
                          fontSize: 12, fontWeight: FontWeight.w500),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  InkWell(
                    onTap: onPlayback,
                    borderRadius: BorderRadius.circular(4),
                    child: Padding(
                      padding: const EdgeInsets.all(2),
                      child: Icon(Icons.folder_open,
                          size: 16,
                          color: cs.onSurface.withOpacity(0.45)),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
