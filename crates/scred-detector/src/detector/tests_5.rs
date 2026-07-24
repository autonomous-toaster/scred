    #[test]
    fn test_redact_mysql_pwd() {
        let text = b"MYSQL_PWD=secretpassword";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Should keep "MYSQL_PWD=" and first 4 of value "secr", redact rest
        assert_eq!(redacted, b"MYSQL_PWD=secrxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_rabbitmq_default_pass() {
        let text = b"RABBITMQ_DEFAULT_PASS=guest123456789";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=22 chars, value=14 chars, preserve first 4 "gues", redact 10
        assert_eq!(redacted, b"RABBITMQ_DEFAULT_PASS=guesxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_redis_password() {
        let text = b"REDIS_PASSWORD=foobared123456";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=15 chars, value=14 chars, preserve first 4 "foob", redact 10
        assert_eq!(redacted, b"REDIS_PASSWORD=foobxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_postgres_password() {
        let text = b"POSTGRES_PASSWORD=postgres_secret_123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=18 chars, value=19 chars, preserve first 4 "post", redact 15
        assert_eq!(redacted, b"POSTGRES_PASSWORD=postxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_docker_registry_password() {
        let text = b"DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=25 chars, value=18 chars, preserve first 4 "dckr", redact 14
        assert_eq!(redacted, b"DOCKER_REGISTRY_PASSWORD=dckrxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

