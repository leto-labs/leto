## ADDED Requirements

### Requirement: Nori Fork Implements ACP Terminal Client Methods

The checked-out Nori fork SHALL implement the ACP terminal client methods used
by ACP agents instead of returning `method_not_found`.

The Nori ACP client bridge SHALL support:

- `terminal/create`
- `terminal/output`
- `terminal/wait_for_exit`
- `terminal/kill`
- `terminal/release`

#### Scenario: ACP agent drives terminal flow through Nori client methods

- **WHEN** an ACP agent requests a client-owned terminal through the Nori ACP
  client bridge
- **THEN** Nori SHALL create the terminal process
- **AND** return output and exit status through the ACP terminal methods

#### Scenario: ACP agent kills a client-owned terminal through Nori

- **WHEN** an ACP agent calls the ACP terminal kill method through Nori
- **THEN** Nori SHALL terminate the terminal command
- **AND** later terminal output or wait calls SHALL reflect terminal exit
