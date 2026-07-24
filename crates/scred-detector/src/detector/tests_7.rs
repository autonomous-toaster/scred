    #[test]
    fn test_redact_couchdb_password() {
        let text = b"COUCHDB_PASSWORD=couchdb_admin_secret";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=17 chars, value=20 chars, preserve first 4 "couc", redact 16
        assert_eq!(redacted, b"COUCHDB_PASSWORD=coucxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_kafka_sasl_password() {
        let text = b"KAFKA_SASL_PASSWORD=kafka_broker_password_24";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=19 chars, value=25 chars, preserve first 4 "kafk", redact 21
        assert_eq!(redacted, b"KAFKA_SASL_PASSWORD=kafkxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_activemq_password() {
        let text = b"ACTIVEMQ_PASSWORD=activemq_broker_secret_pwd";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=18 chars, value=26 chars, preserve first 4 "acti", redact 22
        assert_eq!(redacted, b"ACTIVEMQ_PASSWORD=actixxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_bitbucket_password() {
        let text = b"BITBUCKET_PASSWORD=bitbucket_ci_password_123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=19 chars, value=25 chars, preserve first 4 "bitb", redact 21
        assert_eq!(redacted, b"BITBUCKET_PASSWORD=bitbxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_smtp_password() {
        let text = b"SMTP_PASSWORD=smtp_mail_server_secret";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=13 chars, value=24 chars, preserve first 4 "smtp", redact 20
        assert_eq!(redacted, b"SMTP_PASSWORD=smtpxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

