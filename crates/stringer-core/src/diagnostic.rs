use camino::{Utf8Path, Utf8PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    path: Utf8PathBuf,
    offset: usize,
    len: usize,
}

impl SourceSpan {
    pub fn new(path: impl Into<Utf8PathBuf>, offset: usize, len: usize) -> Self {
        Self {
            path: path.into(),
            offset,
            len,
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: DiagnosticSeverity,
    message: String,
    span: Option<SourceSpan>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            span,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    pub fn span(&self) -> Option<&SourceSpan> {
        self.span.as_ref()
    }
}
