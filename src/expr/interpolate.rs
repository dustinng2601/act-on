//! String interpolation: replace `${{ ... }}` inside arbitrary strings.
//!
//! Mirrors `nektos/act` `Interpolate`: any string with `${{` is rewritten
//! into a `format('...', expr1, expr2, ...)` invocation and evaluated, with
//! the result coerced back to a string.

use super::eval::{Env, Evaluator, Value};
use super::parser;

/// Strip `${` ... `}` wrappers around a pure expression (used for `if:`
/// fields that are full expressions without `{{ }}`).
pub fn strip_expr(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with("${{") && s.ends_with("}}") {
        s[3..s.len() - 2].trim()
    } else {
        s
    }
}

/// Interpolate every `${{ ... }}` in `s` against `env`.
pub fn interpolate(s: &str, env: &Env) -> anyhow::Result<String> {
    if !s.contains("${{") {
        return Ok(s.to_string());
    }

    let evaluator = Evaluator::new(env);
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("${{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let end = after
            .find("}}")
            .ok_or_else(|| anyhow::anyhow!("unterminated ${{ in: {s}"))?;
        let body = after[..end].trim();
        let expr = parser::parse(body)?;
        let v = evaluator.evaluate(&expr, super::eval::DefaultStatusCheck::None)?;
        out.push_str(&v.as_str());
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Interpolate every value inside a [`serde_yaml::Value`] (recursively).
pub fn interpolate_yaml(v: &serde_yaml::Value, env: &Env) -> anyhow::Result<serde_yaml::Value> {
    use serde_yaml::Value;
    Ok(match v {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::Number(n) => Value::Number(n.clone()),
        Value::String(s) => Value::String(interpolate(s, env)?),
        Value::Sequence(seq) => Value::Sequence(
            seq.iter()
                .map(|v| interpolate_yaml(v, env))
                .collect::<anyhow::Result<_>>()?,
        ),
        Value::Mapping(m) => {
            let mut out = serde_yaml::Mapping::new();
            for (k, v) in m.iter() {
                let k = interpolate_yaml(k, env)?;
                let v = interpolate_yaml(v, env)?;
                out.insert(k, v);
            }
            Value::Mapping(out)
        }
        Value::Tagged(t) => Value::Tagged(serde_yaml::value::TaggedValue {
            tag: t.tag.clone(),
            value: interpolate_yaml(t.value(), env)?,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with_env(map: &[(&str, &str)]) -> Env {
        let mut env = Env::default();
        for (k, v) in map {
            env.env.insert((*k).into(), (*v).into());
        }
        env
    }

    #[test]
    fn literals() {
        let env = Env::default();
        assert_eq!(interpolate("hello", &env).unwrap(), "hello");
    }

    #[test]
    fn simple_var() {
        let env = env_with_env(&[("NAME", "world")]);
        assert_eq!(interpolate("hello ${{ env.NAME }}", &env).unwrap(), "hello world");
    }

    #[test]
    fn format_func() {
        let env = env_with_env(&[("NAME", "world")]);
        assert_eq!(
            interpolate("${{ format('hi {0}', env.NAME) }}", &env).unwrap(),
            "hi world"
        );
    }
}
