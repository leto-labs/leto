# OpenAI OAuth Provider

This page covers the OAuth-backed OpenAI provider path in the current
`provider-openai` + shared credential-pool stack.

## Main Pieces

| Component | Role |
| --- | --- |
| `OpenAiOAuthProvider` | Concrete provider using OAuth credentials instead of a static API key |
| `OAuthFlow` | Login helper surface for browser and device-code flows |
| `OpenAiOAuthPreset` | OAuth endpoint, client, and scope metadata |
| `CredentialPool` integration | Lets OAuth-backed providers participate in the same runtime provider discovery path |

## Current Flow

| Step | Behavior |
| --- | --- |
| 1 | User logs in through browser or device flow |
| 2 | Credentials are persisted through the shared credential store |
| 3 | Provider loads or resolves those credentials through the pool |
| 4 | Refresh logic updates access tokens when needed |

## Why It Matters

This is the path that makes first-class login workflows possible without
teaching the runtime to read provider auth directly from environment variables
at execution time.
