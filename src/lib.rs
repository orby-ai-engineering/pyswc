use std::sync::Arc;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use serde::{Serialize, Deserialize};
use swc;
use swc::config::ParseOptions;
use swc_common::{SourceMap, FilePathMapping, FileName, Mark, GLOBALS, comments::{Comments, CommentKind, SingleThreadedComments}};
use swc_ecma_transforms_base::resolver;
use swc_error_reporters::handler::{
    HandlerOpts,
    try_with_handler,
};
use swc_ecma_visit::VisitMutWith;

#[derive(Serialize, Deserialize, Debug)]
struct Span {
    start: u32,
    end: u32,
}

// swc_common::comments::Comment is not serializable, so we convert them here
#[derive(Serialize, Deserialize, Debug)]
struct Comment {
    span: Span,
    kind: String,
    text: String,
}

fn convert_comments(comments: SingleThreadedComments) -> Vec<Comment> {
    let (leading, trailing) = comments.borrow_all();
    leading.values().flatten()
        .chain(trailing.values().flatten())
        .map(|comment| Comment {
            span: Span {
                start: comment.span.lo.0,
                end: comment.span.hi.0,
            },
            kind: if comment.kind == CommentKind::Line { "line".to_string() } else { "block".to_string() },
            text: comment.text.to_string(),
        })
        .collect::<Vec<_>>()
}

/// Parse JavaScript/TypeScript code into AST tree.
#[pyfunction]
fn parse(py: Python<'_>, code: String) -> PyResult<&PyTuple> {
    let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
    let c = Arc::new(swc::Compiler::new(cm));
    let options = ParseOptions::default();
    let filename = FileName::Anon;
    let skip_filename = true;

    let parsed_result =
        GLOBALS.set(&Default::default(), || {
            try_with_handler(
                c.cm.clone(),
                HandlerOpts {
                    skip_filename,
                    ..Default::default()
                },
                |handler| {
                    c.run(|| {
                        let fm = c.cm.new_source_file(filename, code);

                        let comments = SingleThreadedComments::default();
                        let mut p = c.parse_js(
                            fm,
                            handler,
                            options.target,
                            options.syntax,
                            options.is_module,
                            Some(&comments as &dyn Comments),
                        )?;

                        p.visit_mut_with(&mut resolver(
                            Mark::new(),
                            Mark::new(),
                            options.syntax.typescript(),
                        ));

                        Ok((p, comments))
                    })
                },
            )
        });

    if parsed_result.is_err() {
        Err(PyValueError::new_err("Unable to parse the code"))
    } else {
        let (program, comments) = parsed_result.unwrap();
        let serialized = serde_json::to_string(&program);
        let serialized_comments = serde_json::to_string(&convert_comments(comments));
        if serialized.is_err() || serialized_comments.is_err() {
            Err(PyValueError::new_err("Error serializing the AST tree"))
        } else {
            Ok(PyTuple::new(py, [serialized.unwrap(), serialized_comments.unwrap()]))
        }
    }
}

/// Python module for SWC.
#[pymodule]
fn pyswc(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}