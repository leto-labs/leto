## MODIFIED Requirements

### Requirement: Native File Read

The native `file_read` implementation SHALL page output safely for model use.

#### Scenario: Read entire file
- **WHEN** `file_read` is called with `{"path": "foo.txt"}`
- **THEN** it SHALL return a bounded default slice of lines rather than the full
  file
- **AND** if more content remains it SHALL include a continuation hint using
  `offset`

#### Scenario: Read with offset and limit
- **WHEN** `file_read` is called with `{"path": "foo.txt", "offset": 5, "limit": 10}`
- **THEN** it SHALL return lines 5 through 14 with line numbers
- **AND** if more content remains it SHALL include a continuation hint using
  `offset`

#### Scenario: Read truncates long lines
- **WHEN** a returned line exceeds the native line-length cap
- **THEN** the native driver SHALL truncate the displayed line content

### Requirement: Native Shell

The native `shell` implementation SHALL bound returned command output.

#### Scenario: Shell executes command
- **WHEN** `shell` is called with `{"command": "echo hello"}`
- **THEN** it SHALL execute the command and return bounded stdout/stderr

#### Scenario: Shell output exceeds native cap
- **WHEN** combined stdout/stderr exceeds the native output budget
- **THEN** the tool SHALL return a truncated output preview
- **AND** it SHALL include a truncation notice

### Requirement: Native Glob Search

The native `glob_search` implementation SHALL cap large result sets.

#### Scenario: Glob finds matching files
- **WHEN** `glob_search` is called with `{"pattern": "**/*.rs"}`
- **THEN** it SHALL return a bounded list of matching file paths

#### Scenario: Glob result set exceeds native cap
- **WHEN** more than the native maximum number of matches are found
- **THEN** the tool SHALL return only the bounded prefix of matches
- **AND** it SHALL include a truncation notice

### Requirement: Native Grep

The native `grep` implementation SHALL cap large result sets and long match
lines.

#### Scenario: Grep finds matches
- **WHEN** `grep` is called with `{"pattern": "fn main"}`
- **THEN** it SHALL return bounded matching lines with file path and line number

#### Scenario: Grep result set exceeds native cap
- **WHEN** `grep` finds more than the native maximum number of matches
- **THEN** it SHALL return only the bounded prefix of matches
- **AND** it SHALL include a truncation notice

#### Scenario: Grep match line exceeds native cap
- **WHEN** a matching line exceeds the native line-length cap
- **THEN** the tool SHALL truncate the displayed line content

### Requirement: ListDirectory Tool

The native `list_directory` implementation SHALL cap recursive listings.

#### Scenario: Directory listing returns tree output
- **WHEN** `list_directory` is called on a nested directory
- **THEN** it SHALL return a readable tree-like listing
- **AND** omit ignored generated directories

#### Scenario: Directory walk exceeds native entry cap
- **WHEN** a recursive listing discovers more than the native maximum number of
  entries
- **THEN** the tool SHALL stop after the bounded prefix
- **AND** it SHALL include a truncation notice
