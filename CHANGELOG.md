# Changelog

## Unreleased

- Fixed sensor scheduling so a five-second interval writes either the newest ESP32 payload or one `NULL` heartbeat, not both.
- Fixed Redis newest-reading selection.
- Stabilized dashboard sensor values across gaps shorter than one minute.
- Preserved all HTTP GET, POST, and PATCH contracts.
