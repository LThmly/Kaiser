"""Synthetic persisted fixtures for terminal tests; never read a user's library."""
import json
from pathlib import Path


def seed_library(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    directory = str(path.parent.resolve())
    citation = {
        "source_id": 1, "source_name": "replication.md", "line": 1,
        "passage": "Asynchronous replication can let replicas serve stale reads.",
    }

    def empty(name):
        return {"name": name, "directory": directory, "sources": [],
                "messages": [], "cards": [], "notes": ""}

    systems = empty("Distributed systems")
    systems["sources"] = [
        {"id": 1, "name": "replication.md", "selected": True,
         "content": Path(__file__).resolve().parents[1].joinpath("examples/replication.md").read_text()},
        {"id": 2, "name": "consistency-notes.md", "selected": True,
         "content": "Read-your-writes consistency lets clients see their completed writes."},
        {"id": 3, "name": "practice-prompts.md", "selected": False,
         "content": "Explain how replication lag can cause stale reads."},
    ]
    systems["messages"] = [
        {"role": "user", "text": "What is replication lag?", "citations": []},
        {"role": "assistant", "text": citation["passage"], "citations": [citation]},
    ]
    systems["cards"] = [
        {"id": 4 + index, "front": question, "back": "Replicas may receive updates later.",
         "citation": citation, "draft": index == 2, "history": []}
        for index, question in enumerate([
            "Why can replicas return stale reads?", "What is read-your-writes consistency?",
            "How can a profile update avoid stale reads?",
        ])
    ]
    path.write_text(json.dumps({"version": 1, "next_id": 10, "active": 0,
                                "notebooks": [systems, empty("Biology"), empty("Reading")],
                                "review_activity": []}))
