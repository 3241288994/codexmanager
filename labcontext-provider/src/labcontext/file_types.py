"""The single format registry used by direct reads, workspaces and evidence search."""
from pathlib import Path

TEXT_SUFFIXES = {
    ".c", ".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".hxx",
    ".py", ".pyi", ".pyw", ".rs", ".go", ".java", ".kt", ".kts",
    ".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ".mts", ".cts",
    ".vue", ".svelte", ".css", ".scss", ".sass", ".less",
    ".rb", ".php", ".swift", ".m", ".mm", ".cs", ".fs", ".fsx",
    ".r", ".jl", ".lua", ".pl", ".pm", ".scala", ".dart",
    ".sh", ".bash", ".zsh", ".fish", ".ps1", ".psm1", ".bat", ".cmd",
    ".txt", ".md", ".markdown", ".mdx", ".rst", ".adoc", ".tex", ".bib",
    ".json", ".jsonl", ".ndjson", ".yaml", ".yml", ".toml",
    ".csv", ".tsv", ".log", ".xml", ".xsd", ".xsl", ".svg",
    ".ini", ".cfg", ".conf", ".config", ".properties", ".editorconfig",
    ".sql", ".graphql", ".gql", ".proto", ".tf", ".tfvars", ".hcl",
    ".cmake", ".gradle", ".mk", ".dockerfile", ".diff", ".patch",
    ".srt", ".vtt",
}
TEXT_FILE_NAMES = {
    "dockerfile", "containerfile", "gemfile", "rakefile", "license", "licence",
    "copying", "makefile", "gnumakefile", "procfile", "readme", "changelog",
    "justfile", "cmakelists.txt", "requirements.txt", "cargo.lock", "poetry.lock",
    "uv.lock", "pipfile", "pipfile.lock", "yarn.lock", "go.mod", "go.sum",
    ".gitignore", ".gitattributes", ".dockerignore", ".editorconfig",
}
HTML_SUFFIXES = {".html", ".htm"}
PDF_SUFFIXES = {".pdf"}
OFFICE_SUFFIXES = {".docx", ".xlsx", ".pptx"}
NOTEBOOK_SUFFIXES = {".ipynb"}
DOCUMENT_SUFFIXES = HTML_SUFFIXES | PDF_SUFFIXES | OFFICE_SUFFIXES | NOTEBOOK_SUFFIXES
MAX_INSPECT_FILE_BYTES = 10_000_000
INSPECT_VIEWS = {"auto", "outline", "search", "lines", "pages", "json_pointer", "full_bounded"}
SENSITIVE_FILE_SUFFIXES = {".key", ".pem", ".p12", ".pfx"}
SENSITIVE_FILE_NAMES = {
    "id_rsa", "id_ed25519", "credentials.json", "secrets.json", "auth.json",
    ".npmrc", ".pypirc", ".netrc", "admin.token", "codexmanager.rpc-token",
}
DEFAULT_BLOCKED_PARTS = {
    ".git", ".env", ".venv", "__pycache__", "checkpoints", "datasets",
    "node_modules", "site-packages", "tmp", "wandb", ".aws", ".azure",
    ".gnupg", ".kube", ".ssh",
}


def is_sensitive_file(path: Path) -> bool:
    return (path.name.casefold().startswith(".env")
            or path.name.casefold() in SENSITIVE_FILE_NAMES
            or path.suffix.casefold() in SENSITIVE_FILE_SUFFIXES)


def reader_kind(path: Path) -> str | None:
    if is_sensitive_file(path):
        return None
    suffix = path.suffix.casefold()
    if suffix in HTML_SUFFIXES:
        return "html"
    if suffix in PDF_SUFFIXES:
        return "pdf"
    if suffix in OFFICE_SUFFIXES:
        return suffix[1:]
    if suffix in NOTEBOOK_SUFFIXES:
        return "notebook"
    if suffix in TEXT_SUFFIXES or path.name.casefold() in TEXT_FILE_NAMES:
        return "text"
    # Common build variants, without accepting arbitrary extensionless files.
    if path.name.casefold().startswith(("dockerfile.", "containerfile.")):
        return "text"
    return None


def readable_file(path: Path) -> bool:
    return reader_kind(path) is not None
