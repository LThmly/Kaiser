//! Synthetic shared fixtures, compiled only by Rust unit tests.
use crate::model::{Card, Citation, Library, Message, Notebook, Source, current_directory};

const REPLICATION_SOURCE: &str = r#"# Replication, without the mystery

## Asynchronous replication

Replication copies data across multiple nodes. A leader accepts writes and sends updates to replicas. With asynchronous replication, the leader acknowledges a write before every replica has received it. This reduces write latency, but a replica can temporarily serve stale reads.

## Synchronous replication

Synchronous replication waits for an acknowledgement from the required replicas before confirming a write. Waiting increases write latency. If a required replica is unavailable, writes may be delayed or rejected.

## Replication lag

Replication lag is the delay between a write on the leader and its application on a replica. Reading from a lagging replica can return old data. Lag can grow when the network is slow or a replica cannot keep up with writes.

## Read your writes

Read-your-writes consistency ensures you can see your own completed updates. One approach is to route your reads to the leader after you write. Another is to track a write position and only read from replicas that have reached that position.

## A useful tradeoff

Serving reads from replicas distributes load and can put data closer to users. Whether stale data is acceptable depends on the use case. A public view count may tolerate some delay; confirming a just-completed account change usually should reflect the new value.
"#;

impl Library {
    pub fn fixture() -> Self {
        let content = REPLICATION_SOURCE.to_owned();
        let citation = Citation { source_id: 1, source_name: "replication.md".into(), line: 5, passage: "Replication copies data across multiple nodes. A leader accepts writes and sends updates to replicas. With asynchronous replication, the leader acknowledges a write before every replica has received it. This reduces write latency, but a replica can temporarily serve stale reads.".into() };
        let make_card = |id, front: &str, back: &str, draft| Card {
            id,
            front: front.into(),
            back: back.into(),
            citation: Some(citation.clone()),
            draft,
            history: vec![],
        };
        let systems = Notebook {
            name: "Distributed systems".into(),
            directory: current_directory(),
            sources: vec![
                Source { id: 1, name: "replication.md".into(), content, selected: true },
                Source { id: 2, name: "consistency-notes.md".into(), selected: true, content: "# Consistency notes\n\nRead-your-writes consistency ensures a client can see its own completed writes. Routing that client's reads to the leader is one way to achieve it.\n\nEventual consistency means that, if no new updates occur, replicas converge to the same value. It does not promise that every read immediately returns the latest write.\n\nA network partition prevents some nodes from communicating. Systems must decide whether to reject some requests or allow responses that may be inconsistent.\n".into() },
                Source { id: 3, name: "practice-prompts.md".into(), selected: false, content: "# Practice\n\nA user edits their profile and then refreshes the page. The old name appears. Explain how replication lag could cause this and propose a way to provide read-your-writes consistency.\n".into() },
            ],
            messages: vec![
                Message { role: "user".into(), text: "Why can a replica return a stale read?".into(), citations: vec![] },
                Message { role: "assistant".into(), text: "In asynchronous replication, the leader can acknowledge a write before a replica receives the update. A read sent to that replica during the delay can return the older value. [1]\n\nTry explaining what would happen if you updated your profile and immediately refreshed the page.".into(), citations: vec![citation.clone()] },
            ],
            cards: vec![
                make_card(4, "Why can asynchronous replication produce stale reads?", "The leader may acknowledge a write before a replica receives it. Reads from that replica can return an older value.", false),
                make_card(5, "What does read-your-writes consistency guarantee?", "A client can see its own completed writes in subsequent reads.", false),
                make_card(6, "How could you avoid showing an old profile immediately after an edit?", "Route the user's subsequent reads to the leader, or wait until the chosen replica has caught up.", true),
            ],
            notes: "# Replication\n\nUnderstanding: replicas trade freshness for lower latency and availability.\n\nQuestion to revisit: when is a stale read acceptable?\n\nPractice: explain the profile-refresh example without looking at the sources.".into(),
        };
        // Fix the source for the read-your-writes card; all provenance is explicit.
        let mut systems = systems;
        systems.cards[1].citation = Some(Citation { source_id: 2, source_name: "consistency-notes.md".into(), line: 3, passage: "Read-your-writes consistency ensures a client can see its own completed writes. Routing that client's reads to the leader is one way to achieve it.".into() });
        systems.cards[2].citation = Some(Citation { source_id: 1, source_name: "replication.md".into(), line: 17, passage: "Read-your-writes consistency ensures you can see your own completed updates. One approach is to route your reads to the leader after you write. Another is to track a write position and only read from replicas that have reached that position.".into() });
        Self {
            version: 1,
            review_activity: vec![],
            next_id: 7,
            active: 0,
            notebooks: vec![
                systems,
                Notebook::empty("Biology".into()),
                Notebook::empty("Reading room".into()),
            ],
        }
    }
}
