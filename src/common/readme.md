# common cases

## cell

1. methods that logically don't mutate, but need to cache
2. shared ownership with Rc
3. interior mutability in trait methods
4. struct fields that are "implementation details"
5. graph/tree structures with parent nodes

## atomics

- load and store
- fetch and modify
