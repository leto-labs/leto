## ADDED Requirements

### Requirement: Messages May Preserve Assistant Reasoning Content

The shared message model SHALL support optional assistant reasoning content so
 loops may carry it across turns when a provider/loop strategy requires it.

#### Scenario: Assistant reasoning content round-trips through message history

- **WHEN** a loop records assistant reasoning content on a message
- **THEN** later provider calls MAY receive that reasoning content as part of
  the preserved conversation state

### Requirement: Inference Failures Are Classifiable

The shared error/runtime surface SHALL support additive classification for
 inference failures that require different loop recovery behavior.

At minimum, classified inference failures SHALL distinguish:

- retryable generic failures
- context-length exhaustion
- output-length exhaustion

#### Scenario: Loop branches on classified context overflow

- **WHEN** a provider surfaces a context-length overflow through the common
  inference error surface
- **THEN** a loop SHALL be able to trigger summarization-oriented recovery
  without parsing provider-specific free-form error text
