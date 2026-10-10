---
title: "Second post: markdown"
date: 2024-02-01
updated: 2024-02-03
tags: [ignored, because, yaml, keys, are, top-level]
taxonomies:
  tags:
    - kanso
    - markdown
  categories: [Writing, Reference]
extra:
  mood: curious
---

Some of the markdown this port understands.

## Lists

- one
- two
  - two and a half
- three

1. first
2. second

## Quotes

> A quoted line,
> and another.

## Tables

| Name | Count | Note |
|:-----|------:|:----:|
| apples | 3 | `fresh` |
| pears | 12 | ~~old~~ |

---

An image: ![a diagram](/img/logo.svg "Logo"), and a line  
break.

{% note(kind="warning") %}
A body shortcode keeps **its markdown**.
{% end %}
