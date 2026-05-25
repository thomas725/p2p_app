    use super::*;

    #[test]
    fn test_build_broadcast_message() {
        let msg = build_broadcast_message(
            "Hello world".to_string(),
            Some("Alice".to_string()),
            Some("msg-123".to_string()),
        );
        assert_eq!(msg.content, "Hello world");
        assert_eq!(msg.nickname, Some("Alice".to_string()));
        assert_eq!(msg.msg_id, Some("msg-123".to_string()));
        assert!(msg.sent_at.is_some());
    }

    #[test]
    fn test_build_broadcast_message_empty_content() {
        let msg = build_broadcast_message(String::new(), None, None);
        assert!(msg.content.is_empty());
        assert!(msg.nickname.is_none());
        assert!(msg.msg_id.is_none());
    }

    #[test]
    fn test_build_broadcast_message_with_all_fields() {
        let msg = build_broadcast_message(
            "test content".to_string(),
            Some("Tester".to_string()),
            Some("msg-123".to_string()),
        );
        assert_eq!(msg.content, "test content");
        assert_eq!(msg.nickname, Some("Tester".to_string()));
        assert_eq!(msg.msg_id, Some("msg-123".to_string()));
    }
