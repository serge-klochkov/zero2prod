use claim::assert_err;
use zero2prod::domain::subscriber_email::SubscriberEmail;

#[test]
fn empty_string_is_rejected() {
    let email = "".to_string();
    assert_err!(SubscriberEmail::parse(email));
}

#[test]
fn email_missing_at_symbol_is_rejected() {
    let email = "ursuladomain.com".to_string();
    assert_err!(SubscriberEmail::parse(email));
}

#[test]
fn email_missing_subject_is_rejected() {
    let email = "@domain.com".to_string();
    assert_err!(SubscriberEmail::parse(email));
}
