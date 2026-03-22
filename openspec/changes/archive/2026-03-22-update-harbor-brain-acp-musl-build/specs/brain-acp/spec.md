## MODIFIED Requirements

### Requirement: Real Backend Can Bridge File Reads And Writes Through ACP Clients
The real `brain-acp` backend SHALL be able to route file reads and writes
through ACP client-owned filesystem capabilities when running under an ACP
client that provides them.

This path SHALL allow containerized ACP clients such as Harbor to keep file
edits inside the client-managed task workspace rather than writing only to the
backend host filesystem.

For Harbor container-backed runs, the preferred staged backend artifact SHALL be
a portable Linux release build rather than a host-glibc debug build when such
an artifact is available.

#### Scenario: Harbor-backed ACP file operations land in the task workspace
- **WHEN** the real `brain-acp` backend is launched through Harbor's ACP client
- **AND** the model uses the `file_write` or `file_read` tool with a relative path
- **THEN** the file operation SHALL be sent through the ACP client bridge
- **AND** the resolved path SHALL be rooted in the ACP session cwd exposed by the client

#### Scenario: Harbor prefers a portable release artifact
- **WHEN** Harbor launches the real `brain-acp` backend without an explicit
  `backend_artifact_path`
- **THEN** it SHALL prefer a portable Linux release artifact path before
  host-glibc release or debug artifact paths
