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

#[test]
fn native_ordered_graph_transport_and_provenance_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/vcp-graph.json")).expect("native graph");
    let input: ReferenceGraphInput =
        serde_json::from_value(fixture["input"].clone()).expect("neutral graph DTO");
    let graph = reference_graph(&input).expect("reference graph build");
    let value = serde_json::to_value(&graph).unwrap();
    for field in [
        "fact_matrix",
        "transport",
        "wormholes",
        "inbound",
        "provenance",
    ] {
        compare_numeric_subset(&value[field], &fixture["expected"][field], field);
    }
    for range in graph.transport.row_offsets.windows(2) {
        let mass: f64 = graph.transport.weights[range[0]..range[1]].iter().sum();
        assert!(mass <= input.config.outbound_mass + 1e-12);
    }
}

#[test]
fn native_residual_pyramid_projection_handshake_and_features_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-pyramid.json"))
        .expect("native pyramid matrix");
    for case in fixture["cases"].as_array().unwrap() {
        let query: Vec<f32> = serde_json::from_value(case["query"].clone()).unwrap();
        let config: ReferencePyramidConfig =
            serde_json::from_value(case["config"].clone()).unwrap();
        let layers: Vec<Vec<ReferenceResidualCandidate>> =
            serde_json::from_value(case["search_results"].clone()).unwrap();
        let mut level = 0;
        let output = reference_pyramid(&query, &config, |_, limit| {
            let hits = layers.get(level).cloned().unwrap_or_default();
            assert!(hits.len() <= limit);
            level += 1;
            Ok(hits)
        })
        .expect("independent projection");
        compare_numeric_subset(
            &serde_json::to_value(output).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_query_morphology_and_all_omega_components_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-query-shape.json"))
        .expect("query shape matrix");
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceRiverShapeInput =
            serde_json::from_value(case["input"].clone()).unwrap();
        let config: ReferenceOmegaConfig = serde_json::from_value(case["config"].clone()).unwrap();
        let morphology = reference_morphology(&input);
        let omega = reference_omega(&input, &config);
        let total = morphology.atomic_weight
            + morphology.propositional_weight
            + morphology.narrative_weight;
        assert!((total - 1.0).abs() <= 1e-12);
        let actual = serde_json::json!({"morphology":morphology,"omega":omega});
        compare_numeric_subset(&actual, &case["expected"], case["name"].as_str().unwrap());
    }
}

#[test]
fn native_direct_anchor_pool_contacts_and_fallback_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-anchors.json"))
        .expect("native anchor matrix");
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceAnchorInput = serde_json::from_value(case["input"].clone()).unwrap();
        let anchors = reference_anchors(&input);
        compare_numeric_subset(
            &serde_json::to_value(anchors).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_dual_field_path_geometry_components_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/vcp-path.json")).expect("native path matrix");
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferencePathInput = serde_json::from_value(case["input"].clone()).unwrap();
        compare_numeric_subset(
            &serde_json::to_value(reference_path_geometry(&input)).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_relative_topology_alignment_distance_and_source_independence_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-relative-topology.json"))
        .expect("native relative topology");
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceTopologyInput = serde_json::from_value(case["input"].clone()).unwrap();
        compare_numeric_subset(
            &serde_json::to_value(reference_relative_topology(&input)).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_conditional_peer_statistics_role_caps_and_anchor_activation_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-scoring.json"))
        .expect("native scoring matrix");
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceScoreInput = serde_json::from_value(case["input"].clone()).unwrap();
        compare_numeric_subset(
            &serde_json::to_value(reference_v3_scores(&input)).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_candidate_observables_and_pure_score_match_persisted_vectors() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/vcp-observables-pure.json"))
        .expect("native persisted curve matrix");
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceObservableInput =
            serde_json::from_value(case["input"].clone()).unwrap();
        let local: Vec<f32> = serde_json::from_value(case["local_vector"].clone()).unwrap();
        let transfer: Vec<f32> = serde_json::from_value(case["transfer_vector"].clone()).unwrap();
        let topology = serde_json::from_value(case["topology"].clone()).unwrap();
        let morphology = serde_json::from_value(case["morphology"].clone()).unwrap();
        let config = serde_json::from_value(case["config"].clone()).unwrap();
        let observables = reference_observables(&input);
        compare_numeric_subset(
            &serde_json::to_value(&observables).unwrap(),
            &case["expected"]["observables"],
            case["name"].as_str().unwrap(),
        );
        if case["expected"].get("pureScore").is_none() {
            continue;
        }
        let cosine = |a: &[f32], b: &[f32]| {
            let dot: f64 = a
                .iter()
                .zip(b)
                .map(|(a, b)| f64::from(*a) * f64::from(*b))
                .sum();
            let magnitude = |v: &[f32]| v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
            dot / (magnitude(a) * magnitude(b))
        };
        let scores = reference_pure_scores(&ReferencePureInput {
            query_score: cosine(&input.query_vector, &input.curve.chunk_vector),
            local_score: cosine(&local, &input.curve.chunk_vector),
            transfer_score: cosine(&transfer, &input.curve.chunk_vector),
            geometry: &input.geometry,
            observables: &observables,
            topology: &topology,
            morphology: &morphology,
            config: &config,
        });
        compare_numeric_subset(
            &serde_json::json!(scores.pure_score),
            &case["expected"]["pureScore"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_candidate_superset_sources_scores_quotas_and_order_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/vcp-candidate-pool.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let signals: Vec<ReferencePoolSignals> =
            serde_json::from_value(case["input"]["signals"].clone()).unwrap();
        let config: ReferencePoolConfig =
            serde_json::from_value(case["input"]["config"].clone()).unwrap();
        compare_numeric_subset(
            &serde_json::to_value(reference_candidate_pool(&signals, &config)).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}

#[test]
fn native_v3_readout_composes_one_observation_through_candidate_pool_and_final_ranking() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/vcp-v3-readout.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceReadoutInput = serde_json::from_value(case["input"].clone()).unwrap();
        let output = reference_v3_readout(&input).unwrap();
        let actual = serde_json::to_value(output).unwrap();
        for field in ["morphology", "omega", "selected"] {
            compare_numeric_subset(&actual[field], &case["expected"][field], field);
        }
        let expected = case["expected"]["results"].as_array().unwrap();
        let results = actual["results"].as_array().unwrap();
        assert_eq!(results.len(), expected.len());
        for (actual, expected) in results.iter().zip(expected) {
            for field in [
                "id",
                "rank",
                "score",
                "baseScore",
                "geometry",
                "relativeTopology",
                "observables",
                "anchor",
            ] {
                compare_numeric_subset(&actual[field], &expected[field], field);
            }
            for field in [
                "id",
                "role",
                "v2Bonus",
                "gatedBonus",
                "anchorBonus",
                "finalScore",
            ] {
                compare_numeric_subset(
                    &actual["scoring"][field],
                    &expected["scoring"][field],
                    field,
                );
            }
        }
    }
}

#[test]
fn native_dtsc_field_trust_retention_exact_contacts_and_sampling_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/vcp-dtsc-field.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input: ReferenceDtscFieldInput = serde_json::from_value(case["input"].clone()).unwrap();
        compare_numeric_subset(
            &serde_json::to_value(reference_dtsc_field(&input)).unwrap(),
            &case["expected"],
            case["name"].as_str().unwrap(),
        );
    }
}
