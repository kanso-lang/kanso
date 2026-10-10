+++
title = "First post"
description = "Why a generator, and why this one."
[taxonomies]
tags = ["kanso", "intro"]
categories = ["Writing"]
+++

This is the *first* post. It has a summary, which ends here.

<!-- more -->

## Why

A generator turns a tree of markdown into a tree of HTML. That is the whole
job, and it is a good one for finding out what a language is like.

{{ note(kind="tip", text="Shortcodes render through templates.") }}

## How

```rust
fn main() {
    println!("<hello>");
}
```

### A smaller heading

Read the [second post](@/blog/second-post.md#tables) next.
