//! Built-in functions of the GitHub Actions expression language.

use super::ast::Expr;
use super::eval::{Evaluator, Value};
use crate::model::JobStatus;

pub fn call(eval: &Evaluator<'_>, name: &str, args: &[Expr]) -> anyhow::Result<Value> {
    let evaluated: Vec<Value> = args
        .iter()
        .map(|a| eval.eval_inner(a))
        .collect::<anyhow::Result<_>>()?;
    match name {
        "success" => Ok(Value::Bool(eval.env.job.status == JobStatus::Success)),
        "failure" => Ok(Value::Bool(eval.env.job.status == JobStatus::Failure)),
        "cancelled" => Ok(Value::Bool(eval.env.job.status == JobStatus::Cancelled)),
        "always" => Ok(Value::Bool(true)),
        "contains" => {
            if evaluated.len() != 2 {
                anyhow::bail!("contains() expects 2 arguments");
            }
            Ok(Value::Bool(contains(&evaluated[0], &evaluated[1])))
        }
        "startsWith" => {
            if evaluated.len() != 2 {
                anyhow::bail!("startsWith() expects 2 arguments");
            }
            Ok(Value::Bool(
                evaluated[0]
                    .as_str()
                    .to_lowercase()
                    .starts_with(&evaluated[1].as_str().to_lowercase()),
            ))
        }
        "endsWith" => {
            if evaluated.len() != 2 {
                anyhow::bail!("endsWith() expects 2 arguments");
            }
            Ok(Value::Bool(
                evaluated[0]
                    .as_str()
                    .to_lowercase()
                    .ends_with(&evaluated[1].as_str().to_lowercase()),
            ))
        }
        "format" => {
            if evaluated.is_empty() {
                anyhow::bail!("format() expects at least 1 argument");
            }
            let template = evaluated[0].as_str();
            let rest = &evaluated[1..];
            Ok(Value::Str(format_string(&template, rest)))
        }
        "join" => {
            if evaluated.is_empty() || evaluated.len() > 2 {
                anyhow::bail!("join() expects 1 or 2 arguments");
            }
            let sep = evaluated.get(1).map(|v| v.as_str()).unwrap_or_default();
            let arr = match &evaluated[0] {
                Value::Array(a) => a.clone(),
                other => vec![other.clone()],
            };
            Ok(Value::Str(
                arr.iter()
                    .map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(&sep),
            ))
        }
        "toJSON" => {
            if evaluated.len() != 1 {
                anyhow::bail!("toJSON() expects 1 argument");
            }
            Ok(Value::Str(serde_json::to_string_pretty(
                &evaluated[0].to_json(),
            )?))
        }
        "fromJSON" => {
            if evaluated.len() != 1 {
                anyhow::bail!("fromJSON() expects 1 argument");
            }
            let s = evaluated[0].as_str();
            let v: serde_json::Value = serde_json::from_str(&s)?;
            Ok(value_from_json(v))
        }
        "hashFiles" => {
            let patterns: Vec<String> = evaluated.iter().map(|v| v.as_str()).collect();
            let hashes = (eval.env.hash_files)(&patterns);
            Ok(Value::Str(hashes.join("-")))
        }
        other => anyhow::bail!("unknown function: {other}"),
    }
}

fn contains(haystack: &Value, needle: &Value) -> bool {
    match haystack {
        Value::Array(arr) => arr.iter().any(|v| try_eq(v, needle)),
        Value::Str(s) => s.to_lowercase().contains(&needle.as_str().to_lowercase()),
        _ => false,
    }
}

fn format_string(template: &str, args: &[Value]) -> String {
    let mut out = String::new();
    let mut iter = template.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '{' {
            if let Some('{') = iter.peek() {
                out.push('{');
                iter.next();
                continue;
            }
            let mut num = String::new();
            for c in iter.by_ref() {
                if c == '}' {
                    break;
                }
                num.push(c);
            }
            if let Ok(i) = num.parse::<usize>() {
                if let Some(v) = args.get(i) {
                    out.push_str(&v.as_str());
                }
            }
        } else if c == '}' {
            if let Some('}') = iter.peek() {
                out.push('}');
                iter.next();
                continue;
            }
            out.push('}');
        } else {
            out.push(c);
        }
    }
    out
}

fn value_from_json(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(a) => Value::Array(a.into_iter().map(value_from_json).collect()),
        serde_json::Value::Object(o) => Value::Object(
            o.into_iter()
                .map(|(k, v)| (k, value_from_json(v)))
                .collect(),
        ),
    }
}

fn try_eq(a: &Value, b: &Value) -> bool {
    use super::eval::Value::*;
    match (a, b) {
        (Int(x), Float(y)) | (Float(y), Int(x)) => (*x as f64) == *y,
        (Int(x), Int(y)) => x == y,
        (Float(x), Float(y)) => x == y,
        (Bool(x), Bool(y)) => x == y,
        (Str(x), Str(y)) => x.eq_ignore_ascii_case(y),
        (Null, Null) => true,
        _ => false,
    }
}

// Tiny shim: Evaluator needs to expose inner eval to funcs.
impl<'a> Evaluator<'a> {
    pub(crate) fn eval_inner(&self, expr: &Expr) -> anyhow::Result<Value> {
        // Re-route via `evaluate(None)` to keep status-check wrapping off.
        self.evaluate(expr, super::eval::DefaultStatusCheck::None)
    }
}
