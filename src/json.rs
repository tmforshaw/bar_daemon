use std::collections::HashMap;

use tracing::instrument;

use crate::error::DaemonError;

/// # Errors
/// Returns an error if the generated hashmap can't be converted into a JSON
#[instrument]
pub fn tuples_to_json(tuples: Vec<(String, Vec<(String, String)>)>) -> Result<String, DaemonError> {
    // Convert tuples nested hashmap
    let mut json_map: HashMap<String, HashMap<String, String>> = HashMap::new();

    let tuples_locked = tuples;
    for (group, pairs) in tuples_locked {
        let inner_map = pairs.into_iter().collect::<HashMap<_, _>>();

        json_map.insert(group, inner_map);
    }

    Ok(serde_json::to_string(&json_map)?)
}

#[cfg(test)]
#[test]
fn tuple_to_json_test() {
    let tuples = vec![(
        String::from("TestName"),
        vec![(String::from("Param1"), String::from("Param2"))],
    )];

    let json = tuples_to_json(tuples);

    assert!(
        if let Ok(json_string) = json.as_ref() {
            json_string == &"{\"TestName\":{\"Param1\":\"Param2\"}}".to_string()
        } else {
            false
        },
        "{json:?}\n\n{{\"TestName\":{{\"Param1\":\"Param2\"}}}}",
    );
}
