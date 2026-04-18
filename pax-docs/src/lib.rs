use once_cell::sync::Lazy;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser};
use tantivy::schema::{Schema, Value};
use tantivy::TantivyDocument;
use tantivy::{Index, IndexReader};

const MAGIC: &[u8; 8] = b"PAXDOCS\0";
const VERSION: u32 = 2;

static PAXDOCS_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/paxdocs.bin"));

static DATA: Lazy<Result<DocsData, DocsError>> = Lazy::new(load_data);
static INDEX: Lazy<Result<DocsIndex, DocsError>> = Lazy::new(|| {
    let data = data()?;
    DocsIndex::open_prebuilt(data)
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Article,
    Api,
}

impl DocKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocKind::Article => "article",
            DocKind::Api => "api",
        }
    }

    fn from_u8(value: u8) -> Result<Self, DocsError> {
        match value {
            0 => Ok(DocKind::Article),
            1 => Ok(DocKind::Api),
            _ => Err(DocsError::new("Unsupported doc kind")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocEntry {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub kind: DocKind,
    pub depth: u8,
    pub path: String,
    pub body_markdown: String,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub entry_index: usize,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct DocsError {
    message: String,
}

impl DocsError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DocsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DocsError {}

impl From<tantivy::TantivyError> for DocsError {
    fn from(err: tantivy::TantivyError) -> Self {
        DocsError::new(err.to_string())
    }
}

impl From<tantivy::query::QueryParserError> for DocsError {
    fn from(err: tantivy::query::QueryParserError) -> Self {
        DocsError::new(err.to_string())
    }
}

impl From<std::io::Error> for DocsError {
    fn from(err: std::io::Error) -> Self {
        DocsError::new(err.to_string())
    }
}

pub type DocsResult<T> = Result<T, DocsError>;

pub fn entries() -> DocsResult<&'static [DocEntry]> {
    Ok(data()?.entries.as_slice())
}

pub fn find_entry(query: &str) -> DocsResult<Option<&'static DocEntry>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(None);
    }

    let entries = entries()?;
    let query_lower = query.to_lowercase();

    for entry in entries {
        if matches_query(entry, &query_lower) {
            return Ok(Some(entry));
        }
    }

    Ok(None)
}

pub fn search(query: &str, limit: usize) -> DocsResult<Vec<SearchHit>> {
    let index = match &*INDEX {
        Ok(index) => index,
        Err(err) => return Err(err.clone()),
    };

    let query = index.build_query(query)?;
    let searcher = index.reader.searcher();
    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

    let mut hits = Vec::new();
    for (score, doc_address) in top_docs {
        let doc: TantivyDocument = searcher.doc(doc_address)?;
        let entry_index = doc
            .get_first(index.fields.entry_index)
            .and_then(|value| value.as_u64())
            .ok_or_else(|| DocsError::new("Missing entry index in search result"))?;
        hits.push(SearchHit {
            entry_index: entry_index as usize,
            score,
        });
    }

    Ok(hits)
}

struct DocsData {
    entries: Vec<DocEntry>,
    index_files: Vec<IndexFile>,
    index_id: String,
}

struct IndexFile {
    path: String,
    bytes: Vec<u8>,
}

struct DocsIndex {
    index: Index,
    reader: IndexReader,
    fields: DocsIndexFields,
}

#[derive(Clone, Copy)]
struct DocsIndexFields {
    title: tantivy::schema::Field,
    body: tantivy::schema::Field,
    tags: tantivy::schema::Field,
    slug: tantivy::schema::Field,
    path: tantivy::schema::Field,
    entry_index: tantivy::schema::Field,
}

impl DocsIndexFields {
    fn from_schema(schema: &Schema) -> DocsResult<Self> {
        Ok(DocsIndexFields {
            title: get_field(schema, "title")?,
            body: get_field(schema, "body")?,
            tags: get_field(schema, "tags")?,
            slug: get_field(schema, "slug")?,
            path: get_field(schema, "path")?,
            entry_index: get_field(schema, "entry_index")?,
        })
    }
}

impl DocsIndex {
    fn open_prebuilt(data: &DocsData) -> DocsResult<Self> {
        let index_dir = prepare_index_dir(data)?;
        let index = Index::open_in_dir(&index_dir)?;
        let reader = index.reader()?;
        let schema = index.schema();

        Ok(DocsIndex {
            index,
            reader,
            fields: DocsIndexFields::from_schema(&schema)?,
        })
    }

    fn build_query(&self, raw: &str) -> DocsResult<Box<dyn Query>> {
        let mut parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.title,
                self.fields.body,
                self.fields.tags,
                self.fields.slug,
                self.fields.path,
            ],
        );
        parser.set_field_boost(self.fields.title, 3.0);
        parser.set_field_boost(self.fields.tags, 2.0);
        parser.set_field_boost(self.fields.slug, 1.5);

        let parts: Vec<&str> = raw
            .split('|')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect();

        if parts.is_empty() {
            return Err(DocsError::new("Search query is empty"));
        }

        if parts.len() == 1 {
            return Ok(parser.parse_query(parts[0])?);
        }

        let mut queries = Vec::new();
        for part in parts {
            queries.push((Occur::Should, parser.parse_query(part)?));
        }

        Ok(Box::new(BooleanQuery::new(queries)))
    }
}

fn get_field(schema: &Schema, name: &str) -> DocsResult<tantivy::schema::Field> {
    schema
        .get_field(name)
        .map_err(|err| DocsError::new(format!("Missing field '{name}' in docs index: {err}")))
}

fn data() -> DocsResult<&'static DocsData> {
    match &*DATA {
        Ok(data) => Ok(data),
        Err(err) => Err(err.clone()),
    }
}

fn prepare_index_dir(data: &DocsData) -> DocsResult<PathBuf> {
    let base = std::env::temp_dir().join(format!("pax-docs-index-{}", data.index_id));
    let marker = base.join(".paxdocs_complete");
    if marker.exists() {
        return Ok(base);
    }

    fs::create_dir_all(&base)?;
    for file in &data.index_files {
        let path = sanitize_relative_path(&file.path)?;
        let destination = base.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, &file.bytes)?;
    }
    fs::write(marker, data.index_id.as_bytes())?;

    Ok(base)
}

fn sanitize_relative_path(path: &str) -> DocsResult<PathBuf> {
    let candidate = Path::new(path);
    for component in candidate.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(DocsError::new(format!("Invalid index path '{path}'"))),
        }
    }
    Ok(candidate.to_path_buf())
}

fn load_data() -> DocsResult<DocsData> {
    let mut cursor = Cursor::new(PAXDOCS_BYTES);
    let magic = cursor.read_bytes(8)?;
    if magic.as_slice() != MAGIC {
        return Err(DocsError::new("Invalid paxdocs header"));
    }
    let version = cursor.read_u32()?;
    if version != VERSION {
        return Err(DocsError::new("Unsupported paxdocs version"));
    }

    let index_id = cursor.read_string()?;
    let entry_count = cursor.read_u32()? as usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let kind = DocKind::from_u8(cursor.read_u8()?)?;
        let depth = cursor.read_u8()?;
        let slug = cursor.read_string()?;
        let title = cursor.read_string()?;
        let path = cursor.read_string()?;
        let summary = cursor.read_string()?;
        let tag_count = cursor.read_u32()? as usize;
        let mut tags = Vec::with_capacity(tag_count);
        for _ in 0..tag_count {
            tags.push(cursor.read_string()?);
        }
        let body_markdown = cursor.read_string()?;

        entries.push(DocEntry {
            id: slug.clone(),
            title,
            slug,
            kind,
            depth,
            path,
            body_markdown,
            summary,
            tags,
        });
    }

    let file_count = cursor.read_u32()? as usize;
    let mut index_files = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let path = cursor.read_string()?;
        let len = cursor.read_u32()? as usize;
        let bytes = cursor.read_bytes(len)?;
        index_files.push(IndexFile { path, bytes });
    }

    Ok(DocsData {
        entries,
        index_files,
        index_id,
    })
}

fn matches_query(entry: &DocEntry, query_lower: &str) -> bool {
    if entry.slug.eq_ignore_ascii_case(query_lower)
        || entry.path.eq_ignore_ascii_case(query_lower)
        || entry.title.eq_ignore_ascii_case(query_lower)
    {
        return true;
    }

    if let Some(last_segment) = entry.slug.split('/').last() {
        if last_segment.eq_ignore_ascii_case(query_lower) {
            return true;
        }
    }

    false
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read_u8(&mut self) -> DocsResult<u8> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    fn read_u32(&mut self) -> DocsResult<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_string(&mut self) -> DocsResult<String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        let value = std::str::from_utf8(&bytes)
            .map_err(|_| DocsError::new("Invalid utf-8 string in paxdocs"))?;
        Ok(value.to_string())
    }

    fn read_bytes(&mut self, len: usize) -> DocsResult<Vec<u8>> {
        if self.position + len > self.bytes.len() {
            return Err(DocsError::new("Unexpected end of paxdocs"));
        }
        let out = self.bytes[self.position..self.position + len].to_vec();
        self.position += len;
        Ok(out)
    }
}
