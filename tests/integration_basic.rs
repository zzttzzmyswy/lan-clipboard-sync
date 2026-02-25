use lan_clipboard_sync::protocol::{ContentType, ProtocolMessage};
use lan_clipboard_sync::protocol::{decode_message, encode_message};

#[test]
fn protocol_roundtrip_text() {
    let msg = ProtocolMessage::ClipboardUpdate {
        instance_id: "test-instance".into(),
        content_type: ContentType::Text,
        payload_size: 5,
        payload: b"hello".to_vec(),
    };
    let bytes = encode_message(&msg).unwrap();
    let decoded = decode_message(&bytes).unwrap();
    match decoded {
        ProtocolMessage::ClipboardUpdate {
            instance_id,
            content_type,
            payload_size,
            payload,
        } => {
            assert_eq!(instance_id, "test-instance");
            assert!(matches!(content_type, ContentType::Text));
            assert_eq!(payload_size, 5);
            assert_eq!(payload, b"hello");
        }
    }
}

