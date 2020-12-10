extern crate serde_json;
use std::convert::{From, Into, TryInto};
use std::error;
use std::fmt;
use std::fs::File;
use std::io::{self, prelude::*};
use std::iter::{IntoIterator, Iterator};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub enum PugJsonObject {
    Json(serde_json::Value),
    Raw(String),
    Path(PathBuf),
}

impl From<serde_json::Value> for PugJsonObject {
    fn from(json: serde_json::Value) -> PugJsonObject {
        PugJsonObject::Json(json)
    }
}

impl From<PathBuf> for PugJsonObject {
    fn from(path: PathBuf) -> PugJsonObject {
        PugJsonObject::Path(path)
    }
}

impl From<String> for PugJsonObject {
    fn from(raw: String) -> PugJsonObject {
        PugJsonObject::Raw(raw)
    }
}

impl From<&str> for PugJsonObject {
    fn from(raw: &str) -> PugJsonObject {
        PugJsonObject::Raw(raw.into())
    }
}

impl Into<String> for PugJsonObject {
    fn into(self) -> String {
        match self {
            PugJsonObject::Json(value) => format!("'{}'", value),
            PugJsonObject::Raw(value) => value,
            PugJsonObject::Path(value) => String::from(value.to_string_lossy()),
        }
    }
}

pub struct PugOptions {
    version: bool,
    object: Option<PugJsonObject>,
    path: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    no_debug: bool,
    client: bool,
    stdin: bool,
    pretty: bool,
    doctype: Option<String>,
}

impl PugOptions {
    pub fn new() -> Self {
        PugOptions {
            version: false,
            object: None,
            path: None,
            out_dir: None,
            no_debug: false,
            client: false,
            stdin: false,
            pretty: false,
            doctype: None,
        }
    }

    pub fn version(mut self) -> Self {
        self.version = true;
        self
    }

    pub fn with_object(mut self, object: impl Into<PugJsonObject>) -> Self {
        self.object = Some(object.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn out_dir(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(out_dir.into());
        self
    }

    pub fn no_debug(mut self) -> Self {
        self.no_debug = true;
        self
    }

    pub fn client(mut self) -> Self {
        self.client = true;
        self
    }

    pub fn stdin(mut self) -> Self {
        self.stdin = true;
        self
    }

    pub fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }

    pub fn doctype(mut self, dt: String) -> Self {
        self.doctype = Some(dt);
        self
    }
}

impl IntoIterator for PugOptions {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut result: Vec<String> = Vec::new();

        if self.version {
            result.push("--verison".into())
        }

        if let Some(object) = self.object {
            result.push("--obj".into());
            let object: String = object.into();
            result.push(object);
        }

        if let Some(path) = &self.path {
            result.push("--path".into());
            result.push(path.to_string_lossy().into());
        }

        if let Some(out_dir) = &self.out_dir {
            result.push("--out".into());
            result.push(out_dir.to_string_lossy().into());
        }

        if self.pretty {
            result.push("--pretty".into())
        }

        if self.no_debug {
            result.push("--no-debug".into())
        }
        if self.client {
            result.push("--client".into())
        }

        if let Some(doctype) = self.doctype {
            result.push("--doctype".into());
            result.push(doctype);
        }

        result.into_iter()
    }
}

pub enum CompileError {
    Io(std::io::Error),
    PugError(String),
}

impl error::Error for CompileError {}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            CompileError::PugError(pug_error) => write!(f, "{:?}", pug_error),
            CompileError::Io(io_error) => write!(f, "{}", io_error),
        }
    }
}

impl fmt::Debug for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            CompileError::PugError(pug_error) => write!(f, "Pug Error: {}", pug_error),
            CompileError::Io(io_error) => write!(f, "{:?}", io_error),
        }
    }
}
fn process_output(output: io::Result<Output>) -> Result<String, CompileError> {
    match output {
        Ok(output) => {
            if output.stderr.len() > 0 {
                Err(CompileError::PugError(
                    String::from_utf8_lossy(&output.stderr).into(),
                ))
            } else {
                Ok(String::from_utf8_lossy(&output.stdout).into())
            }
        }
        Err(err) => Err(CompileError::Io(err)),
    }
}

pub fn evaluate_with_options(
    file: impl Into<PathBuf>,
    options: PugOptions,
) -> Result<String, CompileError> {
    let options = options.stdin().with_path(file);

    let mut command = Command::new("pug");

    if let Some(path) = &options.path {
        match File::open(path) {
            Ok(file) => {
                command.stdin(file);
                ()
            }
            Err(e) => return Err(CompileError::Io(e)),
        }
    }
    command.args(options);
    process_output(command.output())
}

pub fn evaluate_string_with_options(
    s: String,
    options: PugOptions,
) -> Result<String, CompileError> {
    let options = options.stdin();
    let mut command = Command::new("pug");
    let mut child = command
        .args(options)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| CompileError::Io(e))?;
    let mut stdin = child.stdin.as_mut().unwrap();
    stdin
        .write_all(s.as_bytes())
        .map_err(|e| CompileError::Io(e))?;
    let output = child.wait_with_output();
    process_output(output)
}

pub fn evaluate_string(s: String) -> Result<String, CompileError> {
    let options = PugOptions::new();
    evaluate_string_with_options(s, options)
}

pub fn evaluate(file: impl Into<PathBuf>) -> Result<String, CompileError> {
    let options = PugOptions::new();
    evaluate_with_options(file, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_file() {
        let pug_options = PugOptions::new();
        assert_eq!("<h1>hello pug</h1>", evaluate("test/hello.pug").unwrap());
    }

    #[test]
    fn evaluate_with_string() {
        assert_eq!(
            "<h1>hello pug</h1>",
            evaluate_string(String::from("h1 hello pug")).unwrap()
        );
    }

    #[test]
    fn evaluate_with_string_and_json() {
        assert_eq!(
            "<h1>hello pug</h1>",
            evaluate_string_with_options(
                String::from("h1 hello #{language}"),
                PugOptions::new().with_object(r#"{"language": "pug"}"#)
            )
            .unwrap()
        )
    }
}
