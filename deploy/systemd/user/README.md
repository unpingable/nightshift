# Retired user-level dogfood shelf

The nonportable Watchbill user service and timer were removed during the
canonical runtime cutover. They targeted the retired CLI and carried local
Governor-mode flags, so retaining runnable unit files would have preserved a
second operational path.

Use the canonical system-level reference units one directory above. Historical
context remains in repository history and the explicitly historical design
records; there is no supported user-level unit in this directory.
