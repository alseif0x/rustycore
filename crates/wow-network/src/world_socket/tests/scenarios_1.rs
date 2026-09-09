//! World socket read/write loop regression scenarios, part 1 of 1.
//!
//! Moved out of the world_socket.rs root under #658; every test is unchanged.

use super::*;

#[tokio::test]
async fn session_channels_preserve_capacity_and_socket_pairing() {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener_addr = listener.local_addr().unwrap();
    let connect = TcpStream::connect(listener_addr);
    let (accepted, connected) = tokio::join!(listener.accept(), connect);
    let (server_stream, client_addr) = accepted.unwrap();
    let client_stream = connected.unwrap();

    let mut socket = WorldSocket::new(server_stream, client_addr);
    let (packet_rx, send_tx, write_fence) = socket.create_session_channels();

    assert_eq!(packet_rx.capacity(), Some(256));
    assert_eq!(send_tx.capacity(), Some(256));

    let inbound = [0x34, 0x12, 0xAB];
    socket
        .session_tx
        .as_ref()
        .expect("socket packet sender")
        .send(WorldPacket::from_bytes(&inbound))
        .unwrap();
    assert_eq!(packet_rx.recv_async().await.unwrap().data(), inbound);

    let outbound = vec![0x78, 0x56, 0xCD];
    send_tx.send(outbound.clone()).unwrap();
    assert_eq!(
        socket
            .send_rx
            .as_ref()
            .expect("socket send receiver")
            .recv_async()
            .await
            .unwrap(),
        outbound
    );

    let fence_wait = {
        let write_fence = write_fence.clone();
        let send_tx = send_tx.clone();
        tokio::spawn(async move {
            write_fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_millis(250))
                .await
        })
    };
    let marker = socket
        .send_rx
        .as_ref()
        .expect("socket send receiver")
        .recv_async()
        .await
        .unwrap();
    assert!(
        socket
            .send_write_fence_like_cpp
            .acknowledge_marker_like_cpp(&marker)
    );
    assert_eq!(
        fence_wait.await.unwrap(),
        SocketWriteFenceWaitResultLikeCpp::Written
    );

    drop(client_stream);
}

#[tokio::test]
async fn split_for_io_preserves_socket_compressor_history_like_cpp() {
    use flate2::{Decompress, FlushDecompress};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener_addr = listener.local_addr().unwrap();
    let connect = TcpStream::connect(listener_addr);
    let (accepted, connected) = tokio::join!(listener.accept(), connect);
    let (server_stream, client_addr) = accepted.unwrap();
    let client_stream = connected.unwrap();

    let mut socket = WorldSocket::new(server_stream, client_addr);
    let first_opcode = 0x25E0_u16.to_le_bytes();
    let first_payload = vec![0xA5; compression::COMPRESSION_THRESHOLD + 257];
    let first = socket
        .compressor
        .compress_packet(&first_opcode, &first_payload);
    let second_opcode = 0x2724_u16.to_le_bytes();
    let second_payload = vec![0xA5; compression::COMPRESSION_THRESHOLD + 513];

    let mut uninterrupted = compression::PacketCompressor::new();
    let expected_first = uninterrupted.compress_packet(&first_opcode, &first_payload);
    let expected_second = uninterrupted.compress_packet(&second_opcode, &second_payload);
    assert_eq!(first, expected_first);

    socket.set_encrypt_key([0x42; 16]);
    let (_packet_rx, send_tx, _write_fence) = socket.create_session_channels();
    let (_reader, mut writer) = socket.split_for_io(send_tx);

    let second = writer
        .compressor
        .compress_packet(&second_opcode, &second_payload);
    assert_eq!(
        second, expected_second,
        "split_for_io must transfer the socket's existing deflate stream"
    );

    let mut inflater = Decompress::new(false);
    for (index, (packet, opcode, payload)) in [
        (&first, first_opcode, &first_payload),
        (&second, second_opcode, &second_payload),
    ]
    .into_iter()
    .enumerate()
    {
        let expected_size = i32::from_le_bytes(packet[0..4].try_into().unwrap()) as usize;
        let mut output = vec![0_u8; expected_size + 256];
        let base_out = inflater.total_out() as usize;
        inflater
            .decompress(&packet[12..], &mut output, FlushDecompress::Sync)
            .unwrap_or_else(|error| panic!("packet {index} failed persistent inflate: {error}"));
        let produced = inflater.total_out() as usize - base_out;
        output.truncate(produced);

        assert_eq!(produced, expected_size, "packet {index} decoded size");
        assert_eq!(&output[..2], &opcode, "packet {index} decoded opcode");
        assert_eq!(&output[2..], payload, "packet {index} decoded payload");
    }

    drop(client_stream);
}

#[tokio::test]
async fn write_fence_acknowledges_only_after_prior_fifo_packet_like_cpp() {
    let fence = SocketWriteFenceLikeCpp::default();
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(4);
    let prior_packet = vec![0x23, 0x26, 0xAA];
    send_tx.send(prior_packet.clone()).unwrap();

    let wait_fence = {
        let fence = fence.clone();
        let send_tx = send_tx.clone();
        tokio::spawn(async move {
            fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_millis(250))
                .await
        })
    };

    assert_eq!(send_rx.recv_async().await.unwrap(), prior_packet);
    let marker = send_rx.recv_async().await.unwrap();
    assert!(SocketWriteFenceLikeCpp::marker_id_like_cpp(&marker).is_some());
    assert!(!wait_fence.is_finished());
    assert!(fence.acknowledge_marker_like_cpp(&marker));
    assert_eq!(
        wait_fence.await.unwrap(),
        SocketWriteFenceWaitResultLikeCpp::Written
    );
}

#[tokio::test]
async fn write_fence_waits_for_acknowledgement_within_caller_bound_like_cpp() {
    let fence = SocketWriteFenceLikeCpp::default();
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(2);
    let wait_fence = {
        let fence = fence.clone();
        tokio::spawn(async move {
            fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_secs(1))
                .await
        })
    };

    let marker = send_rx.recv_async().await.unwrap();
    assert!(SocketWriteFenceLikeCpp::marker_id_like_cpp(&marker).is_some());
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!wait_fence.is_finished());
    assert!(fence.acknowledge_marker_like_cpp(&marker));
    assert_eq!(
        wait_fence.await.unwrap(),
        SocketWriteFenceWaitResultLikeCpp::Written
    );
}

#[tokio::test]
async fn write_fence_fails_when_physical_writer_closes_like_cpp() {
    let fence = SocketWriteFenceLikeCpp::default();
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(1);
    let wait_fence = {
        let fence = fence.clone();
        let send_tx = send_tx.clone();
        tokio::spawn(async move {
            fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_secs(1))
                .await
        })
    };

    let marker = send_rx.recv_async().await.unwrap();
    assert!(SocketWriteFenceLikeCpp::marker_id_like_cpp(&marker).is_some());
    fence.close_writer_like_cpp();
    assert_eq!(
        wait_fence.await.unwrap(),
        SocketWriteFenceWaitResultLikeCpp::WriterClosed
    );
    assert!(
        fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
    );

    assert_eq!(
        fence
            .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_secs(1))
            .await,
        SocketWriteFenceWaitResultLikeCpp::WriterClosed
    );
}

#[tokio::test]
async fn write_fence_reports_configured_stalled_writer_timeout_like_cpp() {
    let fence = SocketWriteFenceLikeCpp::default();
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);

    assert_eq!(
        fence
            .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_millis(20),)
            .await,
        SocketWriteFenceWaitResultLikeCpp::TimedOut
    );
    assert!(
        fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
    );
}

#[tokio::test]
async fn cancelled_write_fence_removes_pending_acknowledgement_like_cpp() {
    let fence = SocketWriteFenceLikeCpp::default();
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    send_tx.send(vec![0x23, 0x26, 0xAA]).unwrap();

    let wait_fence = {
        let fence = fence.clone();
        let send_tx = send_tx.clone();
        tokio::spawn(async move {
            fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_secs(30))
                .await
        })
    };

    for _ in 0..100 {
        if !fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        1
    );

    wait_fence.abort();
    assert!(wait_fence.await.unwrap_err().is_cancelled());
    assert!(
        fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
    );
}

#[tokio::test]
async fn cancelled_after_marker_dequeued_still_consumes_marker_without_null_opcode_write_like_cpp()
{
    let fence = SocketWriteFenceLikeCpp::default();
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(1);
    let wait_fence = {
        let fence = fence.clone();
        let send_tx = send_tx.clone();
        tokio::spawn(async move {
            fence
                .wait_for_prior_packets_written_like_cpp(&send_tx, Duration::from_secs(30))
                .await
        })
    };

    // The writer has already dequeued the internal marker. Cancellation
    // now removes only the pending oneshot; it cannot remove this byte
    // sequence from the writer's local variable.
    let marker = send_rx.recv_async().await.unwrap();
    assert!(SocketWriteFenceLikeCpp::marker_id_like_cpp(&marker).is_some());
    assert_eq!(
        fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        1
    );
    wait_fence.abort();
    assert!(wait_fence.await.unwrap_err().is_cancelled());
    assert!(
        fence
            .state
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
    );

    // This is the exact first branch in `SocketWriter::run`: marker
    // recognition is independent of whether an acknowledgement sender is
    // still pending, so the bytes are consumed by `continue` and can
    // never reach `write_encrypted` as NULL_OPCODE.
    let mut bytes_passed_to_writer = Vec::new();
    if !fence.acknowledge_marker_like_cpp(&marker) {
        bytes_passed_to_writer.push(marker);
    }
    assert!(bytes_passed_to_writer.is_empty());
}

#[test]
fn hex_conversion() {
    let bytes = hex_to_bytes("DEADBEEF");
    assert_eq!(bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn hex_empty() {
    let bytes = hex_to_bytes("");
    assert!(bytes.is_empty());
}

#[test]
fn sign_encryption_produces_64_bytes() {
    let key = [0x42u8; 16];
    let sig = sign_enable_encryption(&key, true);
    assert_eq!(sig.len(), 64);

    // Same inputs should produce same signature (Ed25519 is deterministic)
    let sig2 = sign_enable_encryption(&key, true);
    assert_eq!(sig, sig2);
}

#[test]
fn sign_encryption_differs_with_enabled_flag() {
    let key = [0x42u8; 16];
    let sig_on = sign_enable_encryption(&key, true);
    let sig_off = sign_enable_encryption(&key, false);
    assert_ne!(sig_on, sig_off);
}

#[test]
fn connection_init_strings() {
    assert_eq!(SERVER_CONNECTION_INIT.len(), 53);
    assert_eq!(CLIENT_CONNECTION_INIT.len(), 53);
    assert!(SERVER_CONNECTION_INIT.ends_with(b"\n"));
    assert!(CLIENT_CONNECTION_INIT.ends_with(b"\n"));
}

#[test]
fn seeds_are_16_bytes() {
    assert_eq!(AUTH_CHECK_SEED.len(), 16);
    assert_eq!(SESSION_KEY_SEED.len(), 16);
    assert_eq!(CONTINUED_SESSION_SEED.len(), 16);
    assert_eq!(ENCRYPTION_KEY_SEED.len(), 16);
    assert_eq!(ENABLE_ENCRYPTION_SEED.len(), 16);
    assert_eq!(ENABLE_ENCRYPTION_CONTEXT.len(), 16);
}

#[test]
fn private_key_is_32_bytes() {
    assert_eq!(ENTER_ENCRYPTED_MODE_PRIVATE_KEY.len(), 32);
}

#[test]
fn compression_threshold_uses_payload_len_like_cpp() {
    let packet_with_payload = |payload_len: usize| vec![0u8; 2 + payload_len];

    assert!(!should_compress_server_packet_like_cpp(
        &packet_with_payload(compression::COMPRESSION_THRESHOLD - 1)
    ));
    assert!(!should_compress_server_packet_like_cpp(
        &packet_with_payload(compression::COMPRESSION_THRESHOLD)
    ));
    assert!(should_compress_server_packet_like_cpp(
        &packet_with_payload(compression::COMPRESSION_THRESHOLD + 1)
    ));
}

#[test]
fn overspeed_ping_tracker_matches_cpp_threshold() {
    let mut tracker = OverspeedPingTrackerLikeCpp::default();
    let start = Instant::now();

    assert!(!tracker.record_ping(start, 2));
    assert!(!tracker.record_ping(start + Duration::from_secs(1), 2));
    assert!(!tracker.record_ping(start + Duration::from_secs(2), 2));
    assert!(tracker.record_ping(start + Duration::from_secs(3), 2));
}

#[test]
fn overspeed_ping_tracker_resets_after_cpp_window() {
    let mut tracker = OverspeedPingTrackerLikeCpp::default();
    let start = Instant::now();

    assert!(!tracker.record_ping(start, 2));
    assert!(!tracker.record_ping(start + Duration::from_secs(1), 2));
    assert!(!tracker.record_ping(start + Duration::from_secs(28), 2));
    assert!(!tracker.record_ping(start + Duration::from_secs(29), 2));
}

#[test]
fn overspeed_ping_tracker_can_be_disabled_like_cpp() {
    let mut tracker = OverspeedPingTrackerLikeCpp::default();
    let start = Instant::now();

    for offset in 0..10 {
        assert!(!tracker.record_ping(start + Duration::from_secs(offset), 0));
    }
}

#[test]
fn account_ip_lock_rejects_only_when_locked_ip_changes_like_cpp() {
    assert!(!account_ip_lock_rejects_like_cpp(
        false, "10.0.0.1", "10.0.0.2"
    ));
    assert!(!account_ip_lock_rejects_like_cpp(
        true, "10.0.0.1", "10.0.0.1"
    ));
    assert!(account_ip_lock_rejects_like_cpp(
        true, "10.0.0.1", "10.0.0.2"
    ));
}

#[test]
fn account_country_lock_rejects_like_cpp_world_auth() {
    assert!(!account_country_lock_rejects_like_cpp("", "es"));
    assert!(!account_country_lock_rejects_like_cpp("00", "es"));
    assert!(!account_country_lock_rejects_like_cpp("es", ""));
    assert!(!account_country_lock_rejects_like_cpp("es", "es"));
    assert!(account_country_lock_rejects_like_cpp("es", "fr"));
}
