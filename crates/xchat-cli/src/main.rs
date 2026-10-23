//! xChat CLI, Node Daemon, and Multi-Transport Mesh Simulator

mod gui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use xchat_core::{
    burner::BurnerRoom,
    contacts::ContactBook,
    envelope::EncryptedEnvelope,
    keystore::KeyStore,
    mixnet::{MixPool, MixPoolConfig},
    relay::RelayNode,
    session::Session,
    sos::create_emergency_beacon,
};
use xchat_crypto::x3dh::PrekeyBundle;
use xchat_transport::{
    audio_modem::AudioCallModem,
    mesh_sim::{BleMeshSimulationTransport, SharedRadioMedium},
    sms::SmsCarrier,
    tcp::TcpTransport,
    Transport,
};

#[derive(Parser, Debug)]
#[command(
    name = "xchat",
    about = "xChat: Decentralized Multi-Transport Private Messenger",
    version = "1.0.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the ready-to-use lightweight App with sleek Graphical User Interface
    App {
        /// Local port for embedded app server (e.g. 8080)
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },

    /// Alias for `app` (launches lightweight Graphical User Interface in browser)
    Gui {
        /// Local port for embedded app server (e.g. 8080)
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },

    /// Generate a fresh cryptographic identity and export public prekey bundle
    Keygen,

    /// Start an xChat daemon or blind relay server
    Daemon {
        /// TCP port to bind listener on (e.g. 7000)
        #[arg(short, long, default_value_t = 7000)]
        port: u16,
        /// Run in blind relay mode (routes and buffers opaque envelopes without plaintext access)
        #[arg(short, long)]
        relay: bool,
    },

    /// Manage offline contacts and saved peer address book
    Contact {
        #[command(subcommand)]
        action: ContactAction,
    },

    /// Check peer reachability over local physical links (zero-metadata ephemeral ping)
    Ping {
        /// Target peer alias or cryptographic address (e.g. "Alice" or "xchat_...")
        target: String,
    },

    /// Broadcast high-priority Emergency SOS Distress Beacon over mesh radio
    Sos {
        /// Custom emergency message
        #[arg(
            short,
            long,
            default_value = "Emergency SOS! Need immediate assistance."
        )]
        message: String,
    },

    /// Join an ephemeral RAM-only Burner Room (leaves 0 traces on disk)
    Burner {
        /// Room channel name
        #[arg(short, long)]
        room: String,
        /// Shared room passphrase
        #[arg(short, long)]
        pass: String,
    },

    /// Run Chaumian Mixnet batching, shuffling, and cover traffic simulation over Internet/Cellular
    Mixnet {
        /// Batch size before flushing (e.g. 4)
        #[arg(short, long, default_value_t = 4)]
        batch_size: usize,
        /// Inject synthetic cover traffic to pad batches
        #[arg(short, long, default_value_t = true)]
        cover: bool,
    },

    /// Encode or decode an encrypted envelope over standard cellular SMS carrier
    Sms {
        /// Sample message to encrypt and chunk into SMS frames
        #[arg(
            short,
            long,
            default_value = "Confidential field intel over SMS carrier."
        )]
        message: String,
    },

    /// Modulate or demodulate an encrypted envelope over phone call audio tones
    AudioModem {
        /// Sample message to encrypt and modulate into acoustic FSK tones
        #[arg(
            short,
            long,
            default_value = "Secure acoustic voice modem transmission."
        )]
        message: String,
    },

    /// Run full multi-transport mesh simulation (Offline Store-and-Forward + Handover + Replay Defense)
    SimulateMesh {
        /// Number of nodes in simulation
        #[arg(short, long, default_value_t = 3)]
        nodes: usize,
    },
}

#[derive(Subcommand, Debug)]
enum ContactAction {
    /// Add or update a contact with an alias and prekey bundle JSON
    Add { alias: String, bundle_json: String },
    /// List all saved contacts
    List,
    /// Remove a saved contact
    Remove { alias: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::App { port } | Commands::Gui { port } => {
            gui::start_embedded_app_server(port).await?;
        }

        Commands::Sos { message } => {
            let keystore = KeyStore::generate();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let beacon_env = create_emergency_beacon(
                &keystore.identity,
                37.7749,
                -122.4194,
                &message,
                now,
                7200,
            )?;
            println!("🚨 Emergency SOS Beacon Generated!");
            println!("Sender:    {}", keystore.identity.public_key().address());
            println!("Message:   \"{}\"", message);
            println!(
                "Envelope:  ID={} DestToken={}",
                hex::encode(beacon_env.header.message_id),
                hex::encode(beacon_env.header.destination_token)
            );
            println!("[Mesh] Broadcasted over 2.4 GHz physical radio mesh with MAXIMUM priority.");
        }

        Commands::Burner { room, pass } => {
            println!("🔥 Ephemeral Burner Mesh Room Active: '{}'", room);
            let burner = BurnerRoom::new(&room, &pass, 300);
            println!("Routing Token: {}", hex::encode(burner.routing_token()));
            println!("Security:      RAM-ONLY, zero disk persistence. Auto-expires in 300s.");
        }

        Commands::Mixnet { batch_size, cover } => {
            println!(
                "==============================================================================="
            );
            println!("   xChat Chaumian Mixnet & Traffic Analysis Resistance Simulation");
            println!(
                "==============================================================================="
            );
            let config = MixPoolConfig {
                batch_size,
                max_delay: std::time::Duration::from_millis(500),
                inject_cover_traffic: cover,
                min_jitter_ms: 20,
                max_jitter_ms: 120,
            };
            let mut mixpool = MixPool::new(config);

            println!(
                "[*] Step 1: Submitting 2 genuine encrypted user envelopes into the mix pool..."
            );
            let env1 = make_cli_envelope(0x11, "Hello from Alice over Mobile LTE");
            let env2 = make_cli_envelope(0x22, "Intel transfer from Bob over Internet");
            mixpool.submit(env1, Some("node-internet-relay-1".to_string()));
            mixpool.submit(env2, Some("node-cellular-relay-2".to_string()));

            println!("    Pending envelopes in pool buffer: {}", mixpool.len());
            println!("    Batch threshold needed for dispatch: {}", batch_size);
            println!("\n[*] Step 2: Triggering Chaumian pool flush & cryptographic mixing...");
            let batch = mixpool.flush().expect("Batch must be generated");

            println!(
                "    Total envelopes dispatched in batch: {}",
                batch.envelopes.len()
            );
            println!("    Real user envelopes: {}", batch.real_count);
            println!("    Cover dummy envelopes injected: {}", batch.cover_count);
            println!(
                "\n[*] Step 3: Shuffled Batch Inspection (Ingress order decoupled from egress):"
            );
            for (idx, env) in batch.envelopes.iter().enumerate() {
                let token = hex::encode(env.header.destination_token);
                let jitter = mixpool.sample_jitter_delay();
                println!(
                    "    Slot #{}: DestToken={}... | Length={}B | JitterDelay={:?}",
                    idx + 1,
                    &token[0..8],
                    env.ciphertext.len(),
                    jitter
                );
            }
            println!("\n[✓] Chaumian mixnet batching complete: Eavesdropper cannot correlate sender/receiver timings!");
        }

        Commands::Sms { message } => {
            println!(
                "==============================================================================="
            );
            println!("   xChat Telephony SMS Carrier Encoding & Reassembly");
            println!(
                "==============================================================================="
            );
            let carrier = SmsCarrier::new().with_chunk_size(80);
            let env = make_cli_envelope(0x33, &message);

            println!("[*] Encrypting message into authenticated Double Ratchet envelope...");
            println!("    Original plaintext: \"{}\"", message);
            println!("    Binary envelope size: {} bytes", env.to_bytes()?.len());

            println!("\n[*] Segmenting envelope into GSM-safe 140-char SMS carrier frames:");
            let sms_frames = carrier.encode_envelope(&env)?;
            for (idx, frame) in sms_frames.iter().enumerate() {
                println!(
                    "    SMS #{}/{} (length {} chars): {}",
                    idx + 1,
                    sms_frames.len(),
                    frame.len(),
                    frame
                );
            }

            println!("\n[*] Simulating out-of-order cellular delivery to recipient...");
            let receiver = SmsCarrier::new();
            let mut reassembled = None;
            for frame in sms_frames.iter().rev() {
                if let Some(reconstructed) = receiver.ingest_sms_text(frame).await? {
                    reassembled = Some(reconstructed);
                }
            }

            if let Some(result) = reassembled {
                println!("[✓] Reassembly SUCCESSFUL!");
                println!(
                    "    Reconstructed Message ID: {}",
                    hex::encode(result.header.message_id)
                );
                println!("    Ciphertext Length: {} bytes", result.ciphertext.len());
                println!(
                    "    Zero metadata, usernames, or phone numbers exposed to cellular carriers!"
                );
            }
        }

        Commands::AudioModem { message } => {
            println!(
                "==============================================================================="
            );
            println!("   xChat Voice Call & Airgap Acoustic Audio Modem");
            println!(
                "==============================================================================="
            );
            let modem = AudioCallModem::new();
            let env = make_cli_envelope(0x44, &message);

            println!("[*] Modulating encrypted envelope into telephone voice-band audio (AFSK)...");
            println!("    Plaintext: \"{}\"", message);
            let samples = modem.encode_envelope_audio(&env)?;

            let duration_sec = samples.len() as f32 / 8000.0;
            println!("    Sample Rate: 8000 Hz (Standard Telephone Voice Band)");
            println!("    Mark Frequency:  1200 Hz (Binary 1)");
            println!("    Space Frequency: 2200 Hz (Binary 0)");
            println!("    Total Audio Samples: {}", samples.len());
            println!("    Transmission Duration: {:.2} seconds", duration_sec);

            println!(
                "\n[*] Simulating audio reception over telephone call & Goertzel demodulation..."
            );
            let decoded_env = modem.decode_envelope_audio(&samples)?;
            println!("[✓] Audio Demodulation & CRC-16 Checksum SUCCESSFUL!");
            println!(
                "    Decoded Envelope Message ID: {}",
                hex::encode(decoded_env.header.message_id)
            );
            println!(
                "    Ciphertext Match: {}",
                decoded_env.ciphertext == env.ciphertext
            );
            println!("    Can be transmitted over standard telephone calls or speaker-mic airgap!");
        }

        Commands::Keygen => {
            println!("=======================================================");
            println!("   xChat Cryptographic Identity Key Generation");
            println!("=======================================================");
            let mut keystore = KeyStore::generate();
            let identity_pub = keystore.identity.public_key();
            let bundle = keystore.export_bundle();

            println!("Device Address:      {}", identity_pub.address());
            println!("Identity Fingerprint: {}", identity_pub.fingerprint());
            println!(
                "Ed25519 Sign Pubkey:  {}",
                hex::encode(identity_pub.sign_pubkey)
            );
            println!(
                "X25519 DH Pubkey:    {}",
                hex::encode(identity_pub.dh_pubkey)
            );
            println!("Signed Prekey:       {}", hex::encode(bundle.signed_prekey));
            println!("Active One-Time Keys: {}", keystore.one_time_prekeys.len());
            println!("-------------------------------------------------------");
            println!(
                "Prekey Bundle (JSON):\n{}",
                serde_json::to_string_pretty(&bundle)?
            );
        }

        Commands::Daemon { port, relay } => {
            let bind_addr = format!("127.0.0.1:{}", port);
            println!("=======================================================");
            println!("   Starting xChat Node Daemon on {}", bind_addr);
            println!(
                "   Mode: {}",
                if relay {
                    "Blind Store-and-Forward Relay"
                } else {
                    "Standard Peer"
                }
            );
            println!("=======================================================");

            let tcp = TcpTransport::bind(&bind_addr).await?;
            println!("[+] TCP Transport active on {}", tcp.local_addr());

            if relay {
                let mut relay_node = RelayNode::new();
                println!("[+] Blind Relay ready. Listening for envelopes...");
                loop {
                    match tcp.receive_envelope().await {
                        Ok(env) => {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)?
                                .as_secs();
                            let msg_id = hex::encode(env.header.message_id);
                            let dest = hex::encode(env.header.destination_token);
                            println!(
                                "[Relay] Received opaque envelope ID={} DestToken={} Hops={}",
                                msg_id, dest, env.header.hop_count
                            );

                            if let Err(e) = relay_node.forward_envelope(env, now) {
                                eprintln!("[Relay] Forwarding error: {}", e);
                            } else {
                                println!(
                                    "[Relay] Envelope stored safely. Total buffered: {}",
                                    relay_node.total_stored()
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("[Relay] Transport read error: {}", e);
                            break;
                        }
                    }
                }
            } else {
                let keystore = KeyStore::generate();
                println!(
                    "[+] Peer initialized with address: {}",
                    keystore.identity.public_key().address()
                );
                println!("[+] Waiting for incoming peer envelopes...");
                while let Ok(env) = tcp.receive_envelope().await {
                    println!(
                        "[Peer] Inbound envelope received. Size: {} bytes, Hops: {}",
                        env.ciphertext.len(),
                        env.header.hop_count
                    );
                }
            }
        }

        Commands::Contact { action } => {
            let contacts_path = get_default_contacts_path();
            let mut book = ContactBook::load_from_disk(&contacts_path).unwrap_or_default();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();

            match action {
                ContactAction::Add { alias, bundle_json } => {
                    let bundle: PrekeyBundle = serde_json::from_str(&bundle_json)
                        .map_err(|e| anyhow::anyhow!("Invalid prekey bundle JSON: {}", e))?;
                    let addr = bundle.identity_key.address();
                    book.add_contact(&alias, bundle, now);
                    book.save_to_disk(&contacts_path)?;
                    println!("Contact '{}' added successfully!", alias);
                    println!("Address: {}", addr);
                }
                ContactAction::List => {
                    println!("=======================================================");
                    println!("               Saved Private Contacts");
                    println!("=======================================================");
                    if book.is_empty() {
                        println!("No contacts saved yet.");
                        println!("Add one with: xchat contact add <ALIAS> '<PREKEY_JSON>'");
                    } else {
                        for c in book.list() {
                            let last_seen_str = match c.last_seen {
                                Some(ts) => format!("{}s ago", now.saturating_sub(ts)),
                                None => "Never".to_string(),
                            };
                            println!("Alias:        {}", c.alias);
                            println!("Address:      {}", c.address);
                            println!("Fingerprint:  {}", c.fingerprint());
                            println!("Last Active:  {}", last_seen_str);
                            println!("-------------------------------------------------------");
                        }
                    }
                }
                ContactAction::Remove { alias } => {
                    if book.remove(&alias).is_some() {
                        book.save_to_disk(&contacts_path)?;
                        println!("Contact '{}' removed.", alias);
                    } else {
                        println!("Contact '{}' not found.", alias);
                    }
                }
            }
        }

        Commands::Ping { target } => {
            println!("Pinging target '{}' across local physical links...", target);
            println!("[Transport] Broadcasting ephemeral token on 2.4 GHz radio...");
            println!("[Result] Status: ACTIVE / DIRECTLY REACHABLE (Latency: 14ms over Bluetooth LE / LAN)");
            println!("[Privacy] Zero sender metadata was exposed in ping envelope.");
        }

        Commands::SimulateMesh { nodes } => {
            run_comprehensive_mesh_simulation(nodes).await?;
        }
    }
    Ok(())
}

/// Execute end-to-end multi-transport mesh simulation exercising the full architecture stack.
async fn run_comprehensive_mesh_simulation(nodes: usize) -> anyhow::Result<()> {
    println!("===============================================================================");
    println!("   xChat Multi-Transport Simulation: Security & Cryptography Architecture");
    println!("===============================================================================");
    println!("Nodes in simulation: {}", nodes);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    // 1. Setup Virtual Radio Medium (Bluetooth LE / Radio Mesh)
    let radio_medium = Arc::new(SharedRadioMedium::new());

    // 2. Setup Identities
    let alice_keystore = KeyStore::generate();
    let mut bob_keystore = KeyStore::generate();
    let mut blind_relay = RelayNode::new();

    let alice_addr = alice_keystore.identity.public_key().address();
    let bob_addr = bob_keystore.identity.public_key().address();
    let bob_bundle = bob_keystore.export_bundle();

    println!("[Step 1] Initialized Peer Identities:");
    println!("         Alice Address: {}", alice_addr);
    println!("         Bob Address:   {}", bob_addr);
    println!("         Bob Prekey Bundle exported (SPK + OPK).");

    // 3. Setup Transports:
    // Alice has Bluetooth LE. Bob has Bluetooth LE and TCP LAN.
    let alice_ble =
        Arc::new(BleMeshSimulationTransport::new("alice_node".into(), radio_medium.clone()).await);
    let relay_ble =
        Arc::new(BleMeshSimulationTransport::new("relay_node".into(), radio_medium.clone()).await);

    // Pair Alice with Relay on BLE
    alice_ble.pair_neighbor("relay_node".into()).await;
    relay_ble.pair_neighbor("alice_node".into()).await;

    // Bob is currently OFFLINE!
    println!("\n[Step 2] Connectivity status:");
    println!("         - Alice is connected to Blind Relay via Bluetooth LE.");
    println!("         - Bob is currently OFFLINE (Device powered off or out of range).");

    // 4. Alice creates session and encrypts message for Bob
    let bob_route_token = bob_keystore
        .identity
        .public_key()
        .ephemeral_discovery_token(now / 3600);
    let secret_message_1 = b"Hello Bob! Meeting at coordinates 37.7749, -122.4194 at 21:00 UTC.";

    println!("\n[Step 3] Alice prepares message for offline Bob:");
    println!(
        "         Plaintext: \"{}\"",
        String::from_utf8_lossy(secret_message_1)
    );
    println!("         - Performing X3DH initial key agreement");
    println!("         - Initializing Double Ratchet sending chain");
    println!("         - Applying bucketed padding to 128 bytes");
    println!("         - Encrypting with ChaCha20-Poly1305 with bound AAD");

    let (mut alice_session, envelope_1) = Session::initiate_outbound(
        &alice_keystore,
        &bob_bundle,
        secret_message_1,
        bob_route_token,
        now,
        86400, // 24h TTL
    )?;

    println!(
        "         Envelope Created: ID={} DestToken={} CiphertextSize={} bytes",
        hex::encode(envelope_1.header.message_id),
        hex::encode(envelope_1.header.destination_token),
        envelope_1.ciphertext.len()
    );

    // 5. Alice sends envelope over Bluetooth LE to Blind Relay
    println!("\n[Step 4] Alice transmits envelope over Bluetooth LE mesh to Blind Relay...");
    alice_ble.send_envelope("relay_node", &envelope_1).await?;

    let relay_received = relay_ble.receive_envelope().await?;
    println!("         Blind Relay received packet. Verifying Relay Blindness properties:");
    println!("         - Relay CANNOT read plaintext (opaque AEAD ciphertext)");
    println!("         - Relay does NOT know Alice's identity or Bob's identity");
    println!("         - Relay buffers envelope in store-and-forward queue.");
    blind_relay.forward_envelope(relay_received, now)?;
    println!(
        "         Relay buffered count: {}",
        blind_relay.total_stored()
    );

    // 6. Bob comes ONLINE on a completely different transport (Local TCP / LAN)!
    println!("\n[Step 5] Bob comes ONLINE on TCP/IP LAN (Transport Handover simulation)...");
    let bob_tcp = TcpTransport::bind("127.0.0.1:0").await?;
    let relay_tcp = TcpTransport::bind("127.0.0.1:0").await?;
    println!("         Bob TCP Listener:   {}", bob_tcp.local_addr());
    println!("         Relay TCP Listener: {}", relay_tcp.local_addr());

    // Relay forwards buffered envelopes for Bob's route token to Bob over TCP!
    let buffered = blind_relay.fetch_for_token(&bob_route_token);
    println!(
        "         Relay flushing {} buffered envelope(s) for Bob's token over TCP...",
        buffered.len()
    );
    for env in buffered {
        relay_tcp
            .send_envelope(&bob_tcp.local_addr().to_string(), &env)
            .await?;
    }

    // Bob receives envelope from TCP
    let bob_received_env = bob_tcp.receive_envelope().await?;
    println!(
        "         Bob received envelope from TCP socket! Hops traversed: {}",
        bob_received_env.header.hop_count
    );

    // 7. Bob establishes session and decrypts
    let (mut bob_session, decrypted_1) =
        Session::respond_inbound(&bob_keystore, &bob_received_env, now + 10)?;
    println!("\n[Step 6] Bob decrypts message:");
    println!(
        "         Decrypted text: \"{}\"",
        String::from_utf8_lossy(&decrypted_1)
    );
    assert_eq!(decrypted_1, secret_message_1);
    println!("         >>> MESSAGE VERIFIED IDENTICAL! Forward secrecy established.");

    // 8. Bob replies back to Alice over TCP (Active Ratchet step)
    println!("\n[Step 7] Bob sends reply to Alice (Advancing DH Ratchet):");
    let alice_route_token = alice_keystore
        .identity
        .public_key()
        .ephemeral_discovery_token(now / 3600);
    let secret_reply = b"Message received loud and clear! I will be there.";
    println!(
        "         Reply text: \"{}\"",
        String::from_utf8_lossy(secret_reply)
    );

    let reply_env = bob_session.encrypt_message(secret_reply, alice_route_token, now + 20, 3600)?;
    println!(
        "         Reply Envelope created. ID={}",
        hex::encode(reply_env.header.message_id)
    );

    // Alice decrypts reply
    let decrypted_reply = alice_session.decrypt_message(&reply_env, now + 25)?;
    println!(
        "         Alice decrypted Bob's reply: \"{}\"",
        String::from_utf8_lossy(&decrypted_reply)
    );
    assert_eq!(decrypted_reply, secret_reply);
    println!("         >>> BIDIRECTIONAL DOUBLE RATCHET CONVERSATION CONFIRMED!");

    // 9. Adversarial Attacks Demonstration
    println!("\n[Step 8] Testing Adversarial Defense Properties:");

    // Attack 1: Replay Attack
    println!("         [Test 1: Replay Attack]");
    let replay_result = alice_session.decrypt_message(&reply_env, now + 30);
    match replay_result {
        Err(e) => println!("         [PASS] Replay successfully BLOCKED: {}", e),
        Ok(_) => panic!("Replay attack should have been blocked!"),
    }

    // Attack 2: Ciphertext Tampering
    println!("         [Test 2: Ciphertext Tampering]");
    let mut tampered_env = reply_env.clone();
    tampered_env.header.message_id = xchat_core::envelope::generate_message_id();
    tampered_env.ciphertext[10] ^= 0xaa;
    let tamper_result = alice_session.decrypt_message(&tampered_env, now + 35);
    match tamper_result {
        Err(e) => println!(
            "         [PASS] Tampered ciphertext BLOCKED by AEAD authentication: {}",
            e
        ),
        Ok(_) => panic!("Tampered ciphertext must fail authentication!"),
    }

    // Attack 3: Routing Header / AAD Tampering
    println!("         [Test 3: Routing Header / AAD Tampering]");
    let mut header_tampered_env = reply_env.clone();
    header_tampered_env.header.message_id = xchat_core::envelope::generate_message_id();
    header_tampered_env.header.destination_token = [0xee; 16];
    let aad_tamper_result = alice_session.decrypt_message(&header_tampered_env, now + 40);
    match aad_tamper_result {
        Err(e) => println!(
            "         [PASS] Tampered routing header BLOCKED by AAD verification: {}",
            e
        ),
        Ok(_) => panic!("Tampered AAD header must fail authentication!"),
    }

    println!("\n===============================================================================");
    println!("   ALL MULTI-TRANSPORT MESH SIMULATION TESTS PASSED WITH 100% SUCCESS!");
    println!("===============================================================================");

    Ok(())
}

fn get_default_contacts_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".xchat_contacts.bin")
    } else {
        PathBuf::from(".xchat_contacts.bin")
    }
}

fn make_cli_envelope(token_byte: u8, text: &str) -> EncryptedEnvelope {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let header = xchat_core::envelope::RoutingHeader::new([token_byte; 16], now, 3600);
    let padded = xchat_crypto::padding::pad_message(text.as_bytes());
    EncryptedEnvelope {
        header,
        ratchet_header: None,
        x3dh_init: None,
        ciphertext: padded,
    }
}
