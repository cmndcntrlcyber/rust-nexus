//! WS1 Phase 1b integration test: ferry round-trip via mesh gossipsub.
//!
//! Proves: HarnessTask → gossipsub publish on `harness_task(peer)` topic
//! → received by subscriber → handler → result published on `harness_result()`
//! topic → received by originator.
//!
//! This test uses two MeshNodes (operator + agent) communicating over
//! localhost gossipsub to verify the mesh transport path for ferry tasks.

use std::time::Duration;

use nexus_common::NodeIdentity;
use nexus_mesh::node::{MeshEvent, MeshNode};
use nexus_mesh::topics;
use tokio::time::timeout;

#[tokio::test]
async fn mesh_ferry_publish_subscribe_round_trip() {
    let _ = tracing_subscriber::fmt::try_init();

    let operator_identity = NodeIdentity::generate();
    let agent_identity = NodeIdentity::generate();

    // Spawn operator mesh node.
    let operator_handle = MeshNode::spawn(
        &operator_identity,
        "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
    )
    .expect("operator mesh spawn");

    // Wait for operator to start listening.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Get operator's listen address for the agent to dial.
    let operator_addrs = operator_handle.listen_addrs().await;
    assert!(!operator_addrs.is_empty(), "operator should have at least one listen address");

    // Spawn agent mesh node.
    let agent_handle = MeshNode::spawn(
        &agent_identity,
        "/ip4/127.0.0.1/tcp/0".parse().unwrap(),
    )
    .expect("agent mesh spawn");

    // Agent dials operator.
    for addr in &operator_addrs {
        let _ = agent_handle.dial(addr.clone()).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Both subscribe to the ferry result topic.
    let result_topic = libp2p::gossipsub::IdentTopic::new(topics::harness_result());
    operator_handle.subscribe(&result_topic).await.expect("operator sub");
    agent_handle.subscribe(&result_topic).await.expect("agent sub");

    // Agent subscribes to its own task topic.
    let agent_peer_id = agent_handle.local_peer_id().to_string();
    let task_topic = libp2p::gossipsub::IdentTopic::new(topics::harness_task(&agent_peer_id));
    agent_handle.subscribe(&task_topic).await.expect("agent task sub");

    // Operator subscribes to the task topic to verify publish worked.
    operator_handle.subscribe(&task_topic).await.expect("operator task sub");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Operator publishes a harness task via mesh.
    let task_payload = br#"{"task_id":"mesh-001","tool_name":"shell","json_arguments":"{\"command\":\"id\"}"}"#;
    operator_handle
        .publish(&task_topic, task_payload.to_vec())
        .await
        .expect("publish task");

    // Agent should receive the task on the gossipsub topic.
    let received = timeout(Duration::from_secs(5), async {
        loop {
            if let Some(event) = agent_handle.next_event().await {
                if let MeshEvent::GossipMessage { data, topic, .. } = event {
                    if topic == task_topic.hash() {
                        return data;
                    }
                }
            }
        }
    })
    .await
    .expect("agent should receive task within 5s");

    assert_eq!(received, task_payload);

    // Agent publishes a result on the result topic.
    let result_payload = br#"{"task_id":"mesh-001","output":"uid=0(root)","is_error":false}"#;
    agent_handle
        .publish(&result_topic, result_payload.to_vec())
        .await
        .expect("publish result");

    // Operator should receive the result.
    let received_result = timeout(Duration::from_secs(5), async {
        loop {
            if let Some(event) = operator_handle.next_event().await {
                if let MeshEvent::GossipMessage { data, topic, .. } = event {
                    if topic == result_topic.hash() {
                        return data;
                    }
                }
            }
        }
    })
    .await
    .expect("operator should receive result within 5s");

    assert_eq!(received_result, result_payload);
}

#[tokio::test]
async fn mesh_telemetry_snapshot_publish() {
    let _ = tracing_subscriber::fmt::try_init();

    let id1 = NodeIdentity::generate();
    let id2 = NodeIdentity::generate();

    let h1 = MeshNode::spawn(&id1, "/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("spawn 1");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let addrs = h1.listen_addrs().await;
    let h2 = MeshNode::spawn(&id2, "/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("spawn 2");

    for addr in &addrs {
        let _ = h2.dial(addr.clone()).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let telemetry_topic = topics::telemetry_snapshot_topic();
    h1.subscribe(&telemetry_topic).await.expect("sub 1");
    h2.subscribe(&telemetry_topic).await.expect("sub 2");
    tokio::time::sleep(Duration::from_millis(300)).await;

    let snapshot_json = br#"{"window_start":1000,"window_end":2000,"node_features":{}}"#;
    h1.publish(&telemetry_topic, snapshot_json.to_vec())
        .await
        .expect("publish snapshot");

    let received = timeout(Duration::from_secs(5), async {
        loop {
            if let Some(MeshEvent::GossipMessage { data, topic, .. }) = h2.next_event().await {
                if topic == telemetry_topic.hash() {
                    return data;
                }
            }
        }
    })
    .await
    .expect("should receive telemetry snapshot");

    assert_eq!(received, snapshot_json);
}
