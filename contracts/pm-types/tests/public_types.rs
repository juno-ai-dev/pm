use cosmwasm_schema::schema_for;
use cosmwasm_std::{from_json, to_json_binary, Binary, Uint128};
use pm_types::{
    OracleAnswer, Outcome, Payout, ProtocolVersion, PublicTypes, Question, TierId, Ujuno,
    UnixTimestamp, UJUNO_DENOM,
};

#[test]
fn public_types_round_trip_without_losing_wire_meaning() {
    let value = PublicTypes {
        outcome: Outcome::Yes,
        payout: Payout::neutral(),
        version: ProtocolVersion::V1,
        tier: TierId(7),
        question: Question {
            id: Binary::from([0_u8, 1, 0xff]),
            text: "Will the event occur?".to_owned(),
            close_time: UnixTimestamp(1_800_000_000),
            opening_time: UnixTimestamp(1_800_000_001),
        },
        amount: Ujuno(Uint128::new(1_000_000)),
        answer: OracleAnswer(Binary::from([0_u8, 0xff, 1])),
    };

    let encoded = to_json_binary(&value).unwrap();
    let decoded: PublicTypes = from_json(encoded.clone()).unwrap();

    assert_eq!(decoded, value);
    assert_eq!(UJUNO_DENOM, "ujuno");
    assert_eq!(
        String::from_utf8(encoded.to_vec()).unwrap(),
        r#"{"outcome":"yes","payout":{"yes_numerator":"1","no_numerator":"1","denominator":"2"},"version":"v1","tier":7,"question":{"id":"AAH/","text":"Will the event occur?","close_time":1800000000,"opening_time":1800000001},"amount":"1000000","answer":"AP8B"}"#
    );
}

#[test]
fn answer_bytes_are_opaque_and_amounts_require_uint128_strings() {
    let answer: OracleAnswer = from_json(br#""bm90IHllcyBvciBubw==""#).unwrap();
    assert_eq!(answer.0, Binary::from(b"not yes or no"));

    assert!(from_json::<Ujuno>(br#"1000000"#).is_err());
    assert!(from_json::<Ujuno>(br#"-1"#).is_err());
    assert!(from_json::<OracleAnswer>(br#""not base64!""#).is_err());
}

#[test]
fn payout_constructors_are_exact_integer_vectors() {
    assert_eq!(
        Payout::for_outcome(Outcome::Yes),
        Payout {
            yes_numerator: Uint128::one(),
            no_numerator: Uint128::zero(),
            denominator: Uint128::one(),
        }
    );
    assert_eq!(
        Payout::for_outcome(Outcome::No),
        Payout {
            yes_numerator: Uint128::zero(),
            no_numerator: Uint128::one(),
            denominator: Uint128::one(),
        }
    );
    assert_eq!(
        Payout::neutral(),
        Payout {
            yes_numerator: Uint128::one(),
            no_numerator: Uint128::one(),
            denominator: Uint128::new(2),
        }
    );
}

#[test]
fn public_type_schema_matches_tracked_snapshot() {
    let generated = serde_json::to_string_pretty(&schema_for!(PublicTypes)).unwrap() + "\n";
    assert_eq!(generated, include_str!("../schema/pm-types.json"));
}
