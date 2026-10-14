# コミットメッセージ

- コミットメッセージは、件名・本文ともにすべて英語で記述する。
- ユーザーへの返答は日本語で行う。

# TODO Management

`TODO.txt` is the project's inbox for future work, feedback, bugs, and ideas.

## Adding TODOs

When the user asks to remember, note, track, or defer something, add it to `TODO.txt`.

Use one of the following prefixes:

- `[bug]` — Something is not working as intended.
- `[feedback]` — Feedback about existing behavior or implementation.
- `[feature]` — A new feature or capability.
- `[idea]` — An idea that may be considered later.

Keep each TODO concise and on a single line.

Example:

```text
[feedback] Sniper recoil feels too weak
[bug] FOV briefly jumps when exiting ADS
[feature] Add reload behavior
[idea] Consider adding weapon sway
```

## TODOs Are Not Specifications

`TODO.txt` is an inbox, not a source of truth.

Do not treat a TODO item as an implementation requirement.

Do not implement, modify, or fix something merely because it appears in `TODO.txt`.

The specifications under `specs/` are the source of truth for implementation behavior.

## Scope Control

When working on an implementation task, stay within the requested scope.

If you discover an unrelated bug, improvement, refactoring opportunity, or idea:

1. Do not expand the current implementation scope.
2. Add it to `TODO.txt` when it is worth tracking.
3. Continue with the original task.

## Implementing a TODO

When the user explicitly selects a TODO item for implementation:

1. Read the relevant existing specification.
2. Determine whether the TODO requires a specification change.
3. If the specification needs to change, update or propose the specification change before implementation.
4. Implement according to the resulting specification.
5. Verify the implementation.
6. Remove the completed item from `TODO.txt`.

Never silently change the specification to match a TODO.

## General Principle

Maintain this separation:

```text
TODO.txt
    ↓
candidate change / feedback
    ↓
decision
    ↓
specs/
    ↓
implementation
```

`TODO.txt` describes what might change.

`specs/` describes what should exist.

The code implements the specification.
