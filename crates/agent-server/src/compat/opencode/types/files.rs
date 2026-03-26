use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FindTextQueryDoc {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TextMatchFragmentDoc {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindTextSubmatchDoc {
    #[serde(rename = "match")]
    pub matched_text: TextMatchFragmentDoc,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FindTextMatchDoc {
    pub path: TextMatchFragmentDoc,
    pub lines: TextMatchFragmentDoc,
    pub line_number: f64,
    pub absolute_offset: f64,
    pub submatches: Vec<FindTextSubmatchDoc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FindFileQueryDoc {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "FindFileDirsDoc")]
    pub dirs: Option<FindFileDirsDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    #[schemars(with = "FindFileTypeDoc")]
    pub query_type: Option<FindFileTypeDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    #[schemars(range(min = 1, max = 200))]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FindFileDirsDoc {
    #[serde(rename = "true")]
    True,
    #[serde(rename = "false")]
    False,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FindFileTypeDoc {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "directory")]
    Directory,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FindSymbolQueryDoc {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    pub query: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FilePathQueryDoc {
    #[serde(default)]
    #[schemars(with = "String")]
    pub directory: Option<String>,
    #[serde(default)]
    #[schemars(with = "String")]
    pub workspace: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileDiffStatusDoc {
    #[serde(rename = "added")]
    Added,
    #[serde(rename = "deleted")]
    Deleted,
    #[serde(rename = "modified")]
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FileDiff")]
pub struct FileDiffDoc {
    pub file: String,
    pub before: String,
    pub after: String,
    pub additions: f64,
    pub deletions: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FileDiffStatusDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "VcsInfo")]
pub struct VcsInfoDoc {
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Path")]
pub struct PathDoc {
    pub home: String,
    pub state: String,
    pub config: String,
    pub worktree: String,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "File")]
pub struct FileDoc {
    pub path: String,
    #[schemars(range(min = -9007199254740991i64, max = 9007199254740991i64))]
    pub added: i64,
    #[schemars(range(min = -9007199254740991i64, max = 9007199254740991i64))]
    pub removed: i64,
    pub status: FileDiffStatusDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FileNode")]
pub struct FileNodeDoc {
    pub name: String,
    pub path: String,
    pub absolute: String,
    #[serde(rename = "type")]
    pub node_type: FileNodeTypeDoc,
    pub ignored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileNodeTypeDoc {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "directory")]
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FileContent")]
pub struct FileContentDoc {
    #[serde(rename = "type")]
    pub content_type: FileContentTypeDoc,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub diff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "FilePatchDoc")]
    pub patch: Option<FilePatchDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "FileEncodingDoc")]
    pub encoding: Option<FileEncodingDoc>,
    #[serde(default, rename = "mimeType", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileContentTypeDoc {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "binary")]
    Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileEncodingDoc {
    #[serde(rename = "base64")]
    Base64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilePatchDoc {
    #[serde(rename = "oldFileName")]
    pub old_file_name: String,
    #[serde(rename = "newFileName")]
    pub new_file_name: String,
    #[serde(default, rename = "oldHeader", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub old_header: Option<String>,
    #[serde(default, rename = "newHeader", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub new_header: Option<String>,
    pub hunks: Vec<FilePatchHunkDoc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub index: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilePatchHunkDoc {
    #[serde(rename = "oldStart")]
    pub old_start: f64,
    #[serde(rename = "oldLines")]
    pub old_lines: f64,
    #[serde(rename = "newStart")]
    pub new_start: f64,
    #[serde(rename = "newLines")]
    pub new_lines: f64,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Symbol")]
pub struct SymbolDoc {
    pub name: String,
    pub kind: f64,
    pub location: SymbolLocationDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SymbolLocationDoc {
    pub uri: String,
    pub range: RangeDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Range")]
pub struct RangeDoc {
    pub start: PositionDoc,
    pub end: PositionDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PositionDoc {
    pub line: f64,
    pub character: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilePartSourceTextDoc {
    pub value: String,
    #[schemars(range(min = -9007199254740991i64, max = 9007199254740991i64))]
    pub start: i64,
    #[schemars(range(min = -9007199254740991i64, max = 9007199254740991i64))]
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "FileSource")]
pub struct FileSourceDoc {
    pub text: FilePartSourceTextDoc,
    #[serde(rename = "type")]
    pub source_type: FileSourceKindDoc,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FileSourceKindDoc {
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ResourceSource")]
pub struct ResourceSourceDoc {
    pub text: FilePartSourceTextDoc,
    #[serde(rename = "type")]
    pub source_type: ResourceSourceKindDoc,
    #[serde(rename = "clientName")]
    pub client_name: String,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ResourceSourceKindDoc {
    #[serde(rename = "resource")]
    Resource,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "SymbolSource")]
pub struct SymbolSourceDoc {
    pub text: FilePartSourceTextDoc,
    #[serde(rename = "type")]
    pub source_type: SymbolSourceKindDoc,
    pub path: String,
    pub range: RangeDoc,
    pub name: String,
    #[schemars(range(min = -9007199254740991i64, max = 9007199254740991i64))]
    pub kind: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SymbolSourceKindDoc {
    #[serde(rename = "symbol")]
    Symbol,
}
