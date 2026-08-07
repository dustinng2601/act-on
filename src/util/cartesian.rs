//! Cartesian product over matrix dimensions.

use std::collections::HashMap;

use serde_yaml::Value;

/// Take a mapping of `name -> [values...]` and produce every combination.
///
/// Shadows `include` / `exclude` directives are honoured at the runner
/// layer; this function returns the raw cartesian product.
pub fn product(dims: &HashMap<String, Vec<Value>>) -> Vec<HashMap<String, Value>> {
    if dims.is_empty() {
        return vec![HashMap::new()];
    }
    let keys: Vec<String> = dims.keys().cloned().collect();
    let mut iter = keys.iter().map(|k| dims[k].clone());
    let first = iter.next().unwrap();
    let mut combos: Vec<HashMap<String, Value>> = first
        .into_iter()
        .map(|v| {
            let mut m = HashMap::new();
            m.insert(keys[0].clone(), v);
            m
        })
        .collect();
    for (k, vals) in keys.iter().skip(1).zip(iter) {
        let mut next = Vec::new();
        for combo in &combos {
            for v in &vals {
                let mut m = combo.clone();
                m.insert(k.clone(), v.clone());
                next.push(m);
            }
        }
        combos = next;
    }
    combos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_cartesian() {
        let mut d = HashMap::new();
        d.insert("a".into(), vec![Value::String("x".into())]);
        d.insert(
            "b".into(),
            vec![Value::String("y".into()), Value::String("z".into())],
        );
        let out = product(&d);
        assert_eq!(out.len(), 2);
    }
}
