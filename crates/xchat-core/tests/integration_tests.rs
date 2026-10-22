use std::sync::Arc;

use xchat_core::{keystore::KeyStore, relay::RelayNode, session::Session};
use xchat_transport::{
    mesh_sim::{BleMeshSimulationTransport, SharedRadioMedium},
    tcp::TcpTransport,
    Transport,
};

#[tokio::test]
async fn test_multi_hop_blind_relay_delivery() {
    let alice_keystore = KeyStore::generate();
    let mut bob_keystore = KeyStore::generate();
    let bob_bundle = bob_keystore.export_bundle();

    let now = 1_000_000;
    let bob_route_token = [0x55; 16];

    // Alice creates outbound message
    let (mut alice_session, envelope) = Session::initiate_outbound(
        &alice_keystore,
        &bob_bundle,
        b"Secret multi-hop test packet",
        bob_route_token,
        now,
        86400,
    )
    .expect("Alice initiates session");

    // Pass through Relay 1
    let mut relay1 = RelayNode::new();
    relay1
        .forward_envelope(envelope.clone(), now)
        .expect("Relay 1 forwards");
    assert_eq!(relay1.total_stored(), 1);
    let mut from_r1 = relay1.fetch_for_token(&bob_route_token);
    assert_eq!(from_r1.len(), 1);
    let env_r1 = from_r1.remove(0);
    assert_eq!(env_r1.header.hop_count, 1);

    // Pass through Relay 2
    let mut relay2 = RelayNode::new();
    relay2
        .forward_envelope(env_r1, now + 5)
        .expect("Relay 2 forwards");
    let mut from_r2 = relay2.fetch_for_token(&bob_route_token);
    let final_envelope = from_r2.remove(0);
    assert_eq!(final_envelope.header.hop_count, 2);

    // Bob receives from Relay 2 and decrypts
    let (mut bob_session, decrypted) =
        Session::respond_inbound(&bob_keystore, &final_envelope, now + 10)
            .expect("Bob responds inbound");
    assert_eq!(decrypted, b"Secret multi-hop test packet");

    // Bob replies back
    let alice_token = [0xaa; 16];
    let reply_env = bob_session
        .encrypt_message(
            b"Multi-hop response acknowledged",
            alice_token,
            now + 15,
            3600,
        )
        .expect("Bob encrypts reply");

    let alice_decrypted = alice_session
        .decrypt_message(&reply_env, now + 20)
        .expect("Alice decrypts reply");
    assert_eq!(alice_decrypted, b"Multi-hop response acknowledged");
}

#[tokio::test]
async fn test_transport_handover_resilience() {
    // Architecture Rule 45: Changing or losing transport does NOT destroy cryptographic session!
    let medium = Arc::new(SharedRadioMedium::new());
    let alice_ble = Arc::new(BleMeshSimulationTransport::new("alice".into(), medium.clone()).await);
    let bob_ble = Arc::new(BleMeshSimulationTransport::new("bob".into(), medium.clone()).await);

    alice_ble.pair_neighbor("bob".into()).await;
    bob_ble.pair_neighbor("alice".into()).await;

    let alice_keystore = KeyStore::generate();
    let mut bob_keystore = KeyStore::generate();
    let bob_bundle = bob_keystore.export_bundle();

    let now = 2_000_000;
    let bob_token = [0xbb; 16];

    // 1. Session begins over BLE
    let (mut alice_session, env1) = Session::initiate_outbound(
        &alice_keystore,
        &bob_bundle,
        b"Message 1 over Bluetooth LE",
        bob_token,
        now,
        3600,
    )
    .unwrap();

    alice_ble.send_envelope("bob", &env1).await.unwrap();
    let received_ble = bob_ble.receive_envelope().await.unwrap();
    let (mut bob_session, pt1) =
        Session::respond_inbound(&bob_keystore, &received_ble, now + 1).unwrap();
    assert_eq!(pt1, b"Message 1 over Bluetooth LE");

    // 2. Bluetooth radio FAILS / drops connection!
    alice_ble.set_radio_power(false);
    bob_ble.set_radio_power(false);
    assert!(!alice_ble.is_available());
    assert!(!bob_ble.is_available());

    // 3. Fallback to TCP LAN transport
    let alice_tcp = TcpTransport::bind("127.0.0.1:0").await.unwrap();
    let bob_tcp = TcpTransport::bind("127.0.0.1:0").await.unwrap();

    // 4. Send Message 2 over TCP (Double Ratchet session continues seamlessly without reset!)
    let env2 = bob_session
        .encrypt_message(
            b"Message 2 over TCP after Bluetooth dropped!",
            [0xaa; 16],
            now + 5,
            3600,
        )
        .unwrap();

    bob_tcp
        .send_envelope(&alice_tcp.local_addr().to_string(), &env2)
        .await
        .unwrap();
    let received_tcp = alice_tcp.receive_envelope().await.unwrap();

    let pt2 = alice_session
        .decrypt_message(&received_tcp, now + 6)
        .unwrap();
    assert_eq!(pt2, b"Message 2 over TCP after Bluetooth dropped!");
}

#[tokio::test]
async fn test_skipped_keys_and_out_of_order_packets() {
    let alice_keystore = KeyStore::generate();
    let mut bob_keystore = KeyStore::generate();
    let bob_bundle = bob_keystore.export_bundle();

    let now = 3_000_000;
    let (mut alice_session, env_init) = Session::initiate_outbound(
        &alice_keystore,
        &bob_bundle,
        b"Packet 0 (Init)",
        [0xbb; 16],
        now,
        3600,
    )
    .unwrap();

    let (mut bob_session, pt0) =
        Session::respond_inbound(&bob_keystore, &env_init, now + 1).unwrap();
    assert_eq!(pt0, b"Packet 0 (Init)");

    // Alice sends Packets 1, 2, 3, 4
    let env1 = alice_session
        .encrypt_message(b"Packet 1", [0xbb; 16], now + 2, 3600)
        .unwrap();
    let env2 = alice_session
        .encrypt_message(b"Packet 2", [0xbb; 16], now + 3, 3600)
        .unwrap();
    let env3 = alice_session
        .encrypt_message(b"Packet 3", [0xbb; 16], now + 4, 3600)
        .unwrap();
    let env4 = alice_session
        .encrypt_message(b"Packet 4", [0xbb; 16], now + 5, 3600)
        .unwrap();

    // Out-of-order delivery: deliver 4, then 2, then 1, then 3
    let pt4 = bob_session.decrypt_message(&env4, now + 10).unwrap();
    assert_eq!(pt4, b"Packet 4");

    let pt2 = bob_session.decrypt_message(&env2, now + 11).unwrap();
    assert_eq!(pt2, b"Packet 2");

    let pt1 = bob_session.decrypt_message(&env1, now + 12).unwrap();
    assert_eq!(pt1, b"Packet 1");

    let pt3 = bob_session.decrypt_message(&env3, now + 13).unwrap();
    assert_eq!(pt3, b"Packet 3");
}
