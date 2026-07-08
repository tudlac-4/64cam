-- ONVIF event subscription credentials and per-camera motion detection toggle.
ALTER TABLE cameras
  ADD COLUMN onvif_url         TEXT,
  ADD COLUMN onvif_username    TEXT,
  ADD COLUMN onvif_password    TEXT,
  ADD COLUMN motion_detection  BOOLEAN NOT NULL DEFAULT true;
