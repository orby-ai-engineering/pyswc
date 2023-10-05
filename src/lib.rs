use std::sync::Arc;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use swc;
use swc::config::ParseOptions;
use swc_common::{
    SourceMap,
    FilePathMapping,
    FileName,
    Mark,
    GLOBALS,
    comments::{Comments},
};
use swc_ecma_transforms_base::resolver;
use swc_error_reporters::handler::{
    HandlerOpts,
    try_with_handler,
};
use swc_ecma_visit::VisitMutWith;

/// Parse JavaScript/TypeScript code into AST tree.
#[pyfunction]
fn parse(code: String) -> PyResult<String> {
    let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));
    let c = Arc::new(swc::Compiler::new(cm));
    let options = ParseOptions::default();
    let filename = FileName::Anon;
    let skip_filename = true;

    let program =
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

                        let comments = if options.comments {
                            Some(c.comments() as &dyn Comments)
                        } else {
                            None
                        };

                        let mut p = c.parse_js(
                            fm,
                            handler,
                            options.target,
                            options.syntax,
                            options.is_module,
                            comments,
                        )?;

                        p.visit_mut_with(&mut resolver(
                            Mark::new(),
                            Mark::new(),
                            options.syntax.typescript(),
                        ));

                        Ok(p)
                    })
                },
            )
        });

    if program.is_err() {
        Err(PyValueError::new_err("Unable to parse the code"))
    } else {
        let serialized = serde_json::to_string(&program.unwrap());
        if serialized.is_err() {
            Err(PyValueError::new_err("Error serializing the AST tree"))
        } else {
            Ok(serialized.unwrap())
        }
    }
}

/// Python module for SWC.
#[pymodule]
fn pyswc(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}