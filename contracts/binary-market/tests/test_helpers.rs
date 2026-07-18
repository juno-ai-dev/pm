use pm_types::{testing::sample_question, UnixTimestamp};

#[test]
fn shared_question_fixture_uses_unix_seconds_and_opaque_identifiers() {
    let question = sample_question();

    assert_eq!(question.close_time, UnixTimestamp(1_800_000_000));
    assert_eq!(question.opening_time, UnixTimestamp(1_800_000_001));
    assert_eq!(question.id.as_slice(), b"question-id");
}
