    #[test]
    fn test_redact_vault_token() {
        let text = b"VAULT_TOKEN=hvs.CAESIAbcDefG123456HijKl789MnOpQ";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=12 chars, value=35 chars, preserve first 4 "hvs.", redact 31
        assert_eq!(redacted, b"VAULT_TOKEN=hvs.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_ldap_password() {
        let text = b"LDAP_PASSWORD=ldap_admin_password_2024";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=14 chars, value=24 chars, preserve first 4 "ldap", redact 20
        assert_eq!(redacted, b"LDAP_PASSWORD=ldapxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_ldap_bind_password() {
        let text = b"LDAP_BIND_PASSWORD=bind_account_secret_pass";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=19 chars, value=24 chars, preserve first 4 "bind", redact 20
        assert_eq!(redacted, b"LDAP_BIND_PASSWORD=bindxxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_cassandra_password() {
        let text = b"CASSANDRA_PASSWORD=cassandra_node_password";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=19 chars, value=23 chars, preserve first 4 "cass", redact 19
        assert_eq!(redacted, b"CASSANDRA_PASSWORD=cassxxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

    #[test]
    fn test_redact_elasticsearch_password() {
        let text = b"ELASTICSEARCH_PASSWORD=elastic_search_pwd_123";
        let matches = vec![Match::new(0, text.len(), 0)];
        let redacted = redact_text(text, &matches);

        // Key=23 chars, value=22 chars, preserve first 4 "elas", redact 18
        assert_eq!(redacted, b"ELASTICSEARCH_PASSWORD=elasxxxxxxxxxxxxxxxxxx");
        assert_eq!(text.len(), redacted.len());
    }

