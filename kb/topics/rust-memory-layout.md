---
title: Rust Memory Layout and Smart Pointers
slug: rust-memory-layout
tags: [rust, memory, pointers]
---

# Rust Memory Layout: Stack vs. Heap

## 1. The Vec<T> Anatomy
A `Vec<T>` consists of three words on the **stack**:
- `ptr`: Pointer to the data on the **heap**.
- `cap`: Total capacity of the heap allocation.
- `len`: Current number of elements.

When you pass `Vec<T>` by value, you move these three words. The heap data stays put, but ownership changes.

## 2. References vs. Slices
- `&Vec<T>`: A pointer to the stack struct.
- `&[T]` (Slice): A "fat pointer" (ptr + len) directly to the heap data. Idiomatic for read-only access.

## 3. Smart Pointers in Rosemary
- `Box<T>`: Simple heap allocation. Use when you have a large struct or need a stable address.
- `Arc<T>`: Atomic Reference Counting. Used in `src/vector.rs` to share `Arrow` schemas across threads safely.

## 4. Hardening Tip
If a function doesn't need to resize or own the vector, prefer `&[T]` over `&Vec<T>`. It's more flexible (works with arrays, vectors, and sub-slices).
