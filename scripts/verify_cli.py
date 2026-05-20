import subprocess
import json
import os
import shutil
from pathlib import Path

def run_cmd(args):
    print(f"Running: {' '.join(args)}")
    return subprocess.run(args, capture_output=True, text=True)

def main():
    # 0. Clean up previous test state
    if os.path.exists(".rosemary"):
        shutil.rmtree(".rosemary")
    os.makedirs(".rosemary/topics", exist_ok=True)
    
    # Ensure debug binary exists
    print("Building debug binary...")
    subprocess.run(["cargo", "build"], check=True)
    exe = "./target/debug/rosemary"

    # 1. Simulate Agent A: Setting up Stable Knowledge
    print("\n--- Agent A: Setting up Stable Knowledge ---")
    run_cmd([exe, "add-entity", "rosemary", "project"])
    run_cmd([exe, "add-observation", "rosemary", "A CLI-first Knowledge Graph for agents"])
    run_cmd([exe, "add-entity", "KISS", "standard"])
    run_cmd([exe, "add-observation", "KISS", "Keep It Simple, Stupid - prioritize simplicity"])
    run_cmd([exe, "relate", "rosemary", "KISS", "follows"])
    print("Agent A setup OK")

    # 2. Simulate Agent A: Storing Volatile Session State (End of Session 1)
    print("\n--- Agent A: Saving Volatile Session State ---")
    session_name = "rosemary:session"
    run_cmd([exe, "add-entity", session_name, "session"])
    run_cmd([exe, "add-observation", session_name, "Task: Implement deletion commands"])
    run_cmd([exe, "add-observation", session_name, "Status: IN_PROGRESS"])
    run_cmd([exe, "compact"]) # Archive to Markdown for durability
    print("Session 1 archival OK")

    # 3. Simulate Agent B: Resuming from Shared Memory (Session 2)
    print("\n--- Agent B: Resuming from Shared Memory ---")
    # Agent B queries for the previous session status
    res = run_cmd([exe, "query", "status"])
    assert "rosemary:session" in res.stdout
    assert "IN_PROGRESS" in res.stdout
    
    # Agent B checks the full list to understand the graph
    res = run_cmd([exe, "list"])
    assert "rosemary --(follows)--> KISS" in res.stdout
    print("Agent B context recovery OK")

    # 4. Simulate Agent B: Updating Volatile State (Handing over to Session 3)
    print("\n--- Agent B: Updating Volatile State ---")
    # Agent B completes the task and updates status
    # First, it deletes the old status observation
    run_cmd([exe, "delete-observation", session_name, "Status: IN_PROGRESS"])
    run_cmd([exe, "add-observation", session_name, "Status: DONE"])
    run_cmd([exe, "add-observation", session_name, "New Task: Enhance verification scripts"])
    run_cmd([exe, "compact"])
    print("Session 2 update OK")

    # 5. Verify the Final State (Archived Markdown)
    print("\n--- Final Verification: Markdown Durability ---")
    with open(".rosemary/topics/rosemary-session.md", "r") as f:
        content = f.read()
        assert "Status: DONE" in content
        assert "New Task: Enhance verification scripts" in content
        assert "Status: IN_PROGRESS" not in content
    
    with open(".rosemary/topics/rosemary.md", "r") as f:
        content = f.read()
        assert "A CLI-first Knowledge Graph for agents" in content
        assert "KISS" in content or "kiss" in content # Relation check
    print("Markdown durability check OK")

    # 6. Test Query over Large Context (Semantic Search)
    print("\n--- Testing Semantic Recall across sessions ---")
    res = run_cmd([exe, "query", "how to keep it simple"])
    assert "KISS" in res.stdout
    
    res = run_cmd([exe, "query", "what was implemented in session 2"])
    assert "rosemary:session" in res.stdout
    print("Semantic recall OK")

    print("\n✅ All business flow scenarios (Stable vs Volatile, Shared Memory, Archival) verified!")

if __name__ == "__main__":
    main()
