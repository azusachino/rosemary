---
name: rosemary
description: Use Rosemary to store and retrieve long-term project memory (Entities, Observations, Relations) via CLI.
---

# Rosemary Memory Skill

Use Rosemary to maintain a persistent Knowledge Graph of project facts, user preferences, and technical decisions.

## When to use
- **New Fact**: When you learn something stable about the project.
- **Preference**: When the user expresses a tool or style preference.
- **Context Search**: When you need to recall previous decisions or related entities.

## Commands

### 1. Store Knowledge
- **Add Entity**: `rosemary add-entity <name> <type>` (Types: project, preference, standard, concept)
- **Add Observation**: `rosemary add-obs <name> <content>`
- **Relate**: `rosemary relate <from> <to> <relation>`

### 2. Retrieve Knowledge
- **Search**: `rosemary query "<search_term>"` (Vector + FTS search)
- **List Graph**: `rosemary list` (See everything)

### 3. Maintenance
- **Archive**: `rosemary compact` (Syncs DB memory to durable Markdown files in `.rosemary/topics`)

## Protocol
1. **Prefer `rosemary query`** first if you think context might exist.
2. **Batch updates**: Use `rosemary add-obs` frequently to keep the session state fresh.
3. **Relationships**: Always link related concepts (e.g., `rosemary relate "anyhow" "rust" "error-handling"`).
