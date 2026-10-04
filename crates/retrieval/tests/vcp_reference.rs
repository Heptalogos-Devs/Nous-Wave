use nous_retrieval::reference::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct FixtureInput {
    epa: ReferenceEpaInput,
    transport: ReferenceTransport,
    source_field: Vec<(i64, f64)>,
    config: ReferenceFieldConfig,
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-10 + 1e-10 * b.abs(), "{a} != {b}");
}
fn field(actual: &[(i64, f64)], expected: &Value) {
    let expected: Vec<(i64, f64)> = serde_json::from_value(expected.clone()).expect("native field");
    assert_eq!(actual.len(), expected.len());
    for ((a, av), (b, bv)) in actual.iter().zip(expected) {
        assert_eq!(*a, b);
        near(*av, bv);
    }
}
#[test]
fn native_epa_and_dual_field_intermediate_values_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-epa-dual-fields.json"))
        .expect("numeric fixture");
    let input: FixtureInput =
        serde_json::from_value(fixture["input"].clone()).expect("neutral DTO");
    let expected = &fixture["expected"];
    let epa = reference_epa_analysis(&input.epa).expect("EPA");
    for (key, value) in [
        ("logic_depth", epa.logic_depth),
        ("entropy", epa.entropy),
        ("resonance", epa.resonance),
    ] {
        near(value, expected["epa"][key].as_f64().expect("scalar"));
    }
    for axis in expected["epa"]["axis_probabilities"]
        .as_array()
        .expect("axes")
    {
        near(
            epa.axis_probabilities[axis["index"].as_u64().expect("index") as usize],
            axis["energy"].as_f64().expect("energy"),
        );
    }
    let fields = reference_dual_fields(&input.transport, &input.source_field, &input.config)
        .expect("dual fields");
    field(&fields.local_field, &expected["local_field"]);
    field(&fields.transfer_field, &expected["transfer_field"]);
    assert_eq!(
        serde_json::to_value(fields.local_domain).unwrap(),
        expected["local_domain"]
    );
    assert_eq!(
        serde_json::to_value(fields.transfer_domain).unwrap(),
        expected["transfer_domain"]
    );
    let diagnostics = &expected["diagnostics"];
    assert_eq!(
        fields.iterations,
        diagnostics["iterations"].as_u64().unwrap() as usize
    );
    assert_eq!(
        fields.local_converged,
        diagnostics["localConverged"].as_bool().unwrap()
    );
    assert_eq!(
        fields.transfer_converged,
        diagnostics["transferConverged"].as_bool().unwrap()
    );
    near(
        fields.local_residual,
        diagnostics["localResidual"].as_f64().unwrap(),
    );
    near(
        fields.transfer_residual,
        diagnostics["transferResidual"].as_f64().unwrap(),
    );
    let empty = reference_dual_fields(&input.transport, &[], &input.config).expect("empty source");
    assert_eq!(empty.iterations, 0);
    assert!(empty.local_field.is_empty() && empty.transfer_field.is_empty());
}

fn compare_numeric_subset(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            let a = a.as_f64().unwrap();
            let b = b.as_f64().unwrap();
            assert!(
                (a - b).abs() <= 1e-12 + 1e-12 * b.abs(),
                "{path}: {a} != {b}"
            );
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}");
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                compare_numeric_subset(a, b, &format!("{path}/{index}"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                compare_numeric_subset(
                    value,
                    b.get(key).unwrap_or_else(|| panic!("{path}/{key}")),
                    &format!("{path}/{key}"),
                );
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

#[test]
fn frozen_sense_graph_matrix_matches_intermediate_numeric_and_discrete_contracts() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/vcp-sense.json")).expect("frozen Sense matrix");
    for case in fixture["cases"].as_array().expect("cases") {
        let graph: ReferenceSenseGraph =
            serde_json::from_value(case["graph"].clone()).expect("neutral graph");
        let input: ReferenceSenseInput =
            serde_json::from_value(case["input"].clone()).expect("neutral query");
        let output = reference_sense(&graph, &input).expect("independent Sense");
        let mut actual = serde_json::to_value(output).expect("numeric output");
        let mut expected = case["expected"].clone();
        // Flow/node array ties in frozen VCP lack a total comparator. Canonical
        // key order compares every value and the exact discrete membership.
        for value in [&mut actual, &mut expected] {
            value["nodes"]
                .as_array_mut()
                .unwrap()
                .sort_by_key(|n| n["id"].as_i64().unwrap());
            value["edges"].as_array_mut().unwrap().sort_by_key(|e| {
                (
                    e["sourceId"].as_i64().unwrap(),
                    e["targetId"].as_i64().unwrap(),
                )
            });
            value["sourceField"]
                .as_array_mut()
                .unwrap()
                .sort_by_key(|entry| entry[0].as_i64().unwrap());
        }
        compare_numeric_subset(&actual, &expected, case["name"].as_str().unwrap());
    }
}
