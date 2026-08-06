---
id: TASK_PLAN_0001
state: completed
phase: done
created_at: 2026-08-06T14:52:14.114Z
updated_at: 2026-08-06T14:56:47.305Z
title: when display the recent status, check an entery is in a thread, add the indicator if it is in a thread
---

## task prompt

when display the recent status, check an entery is in a thread, add the indicator if it is in a thread

## decisions

Q1: Should the "in a thread" indicator be applied only to replies (statuses with an in_reply_to_id), or also to root statuses that have been replied to within the fetched list? This decides whether the logic only checks in_reply_to_id or requires adding the id field to Status to cross-reference replies.
A1: apply it only to replies and use the 🧵 emoji
Q2: Where should the 🧵 indicator be placed in the output line? This determines if the thread marker appears as a prefix to the content or as a suffix alongside the image indicator.
A2: place it before the content (e.g., 1. 🧵 Content 🖼️)

## notes

Q: show the plan
A: Add `in_reply_to_id: Option<String>` to the `Status` struct in `src/main.rs`.
In the recent statuses loop in `src/main.rs`, determine a thread indicator:
- Set it to `🧵 ` if `status.in_reply_to_id` is `Some`.
- Otherwise, set it to an empty string.
Update the `println!` call to place this indicator before the cleaned content.
──────────────────────────────────
↑4.5k  ↓623  ·  $0.0000  ·  󰅐 5.7s

## handoff

handoff_at: 2026-08-06T14:56:47.302Z
decisions: 2
