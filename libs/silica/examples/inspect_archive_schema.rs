use plist::Value;
use silica::limits::MAX_DOCUMENT_ARCHIVE_BYTES;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::ExitCode,
};
use zip::ZipArchive;

const MAX_INITIAL_READ_CAPACITY: u64 = 16 * 1024 * 1024;
const MAX_REPORTED_VALUES: usize = 8;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let input = args.next().map(PathBuf::from).ok_or(
        "usage: inspect_archive_schema <document-or-directory> \
             [--all-keys] [--continue-on-error] [--skip-offline]",
    )?;
    let mut show_all_keys = false;
    let mut continue_on_error = false;
    let mut skip_offline = false;
    for arg in args {
        match arg.to_str() {
            Some("--all-keys") => show_all_keys = true,
            Some("--continue-on-error") => continue_on_error = true,
            Some("--skip-offline") => skip_offline = true,
            _ => {
                return Err("usage: inspect_archive_schema <document-or-directory> \
                     [--all-keys] [--continue-on-error] [--skip-offline]"
                    .into());
            }
        }
    }

    let paths = collect_paths(&input)?;
    if paths.is_empty() {
        return Err(format!("no .procreate files found under {}", input.display()).into());
    }

    let mut summary = SchemaSummary::default();
    for (index, path) in paths.iter().enumerate() {
        if skip_offline && is_offline(path)? {
            summary.files_skipped += 1;
            eprintln!("skipping_offline={}", path.display());
            continue;
        }
        eprintln!(
            "scanning={}/{}|file={}",
            index + 1,
            paths.len(),
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<non-UTF-8 filename>")
        );
        if let Err(error) = inspect_path(path, &mut summary) {
            if !continue_on_error {
                return Err(error);
            }
            summary.files_failed += 1;
            eprintln!("scan_error={}|error={error}", path.display());
        }
    }
    summary.files_discovered = paths.len();
    summary.print(show_all_keys);
    Ok(())
}

fn collect_paths(input: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if input.is_file() {
        return Ok(vec![input.to_owned()]);
    }
    if !input.is_dir() {
        return Err(format!("input does not exist: {}", input.display()).into());
    }

    let mut paths = std::fs::read_dir(input)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("procreate"))
        })
        .collect::<Vec<_>>();
    paths.sort_unstable();
    Ok(paths)
}

#[cfg(windows)]
fn is_offline(path: &Path) -> Result<bool, Box<dyn Error>> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    let attributes = path.metadata()?.file_attributes();
    Ok(attributes & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS) != 0)
}

#[cfg(not(windows))]
fn is_offline(_: &Path) -> Result<bool, Box<dyn Error>> {
    Ok(false)
}

fn inspect_path(path: &Path, summary: &mut SchemaSummary) -> Result<(), Box<dyn Error>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut document = archive.by_name("Document.archive")?;
    let declared_size = document.size();
    if declared_size > MAX_DOCUMENT_ARCHIVE_BYTES {
        return Err(format!(
            "{} Document.archive exceeds the {}-byte limit",
            path.display(),
            MAX_DOCUMENT_ARCHIVE_BYTES
        )
        .into());
    }

    let initial_capacity = usize::try_from(declared_size.min(MAX_INITIAL_READ_CAPACITY))?;
    let mut bytes = Vec::with_capacity(initial_capacity);
    document
        .by_ref()
        .take(MAX_DOCUMENT_ARCHIVE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOCUMENT_ARCHIVE_BYTES {
        return Err(format!(
            "{} Document.archive exceeded the {}-byte limit while reading",
            path.display(),
            MAX_DOCUMENT_ARCHIVE_BYTES
        )
        .into());
    }

    let archive = Value::from_reader(Cursor::new(bytes))?
        .into_dictionary()
        .ok_or("Document.archive root is not a dictionary")?;
    let objects = archive
        .get("$objects")
        .and_then(Value::as_array)
        .ok_or("$objects is missing or is not an array")?;
    let root = archive
        .get("$top")
        .and_then(Value::as_dictionary)
        .and_then(|top| top.get("root"))
        .and_then(|value| dereference(value, objects))
        .and_then(Value::as_dictionary)
        .ok_or("$top.root does not resolve to a dictionary")?;

    summary.files_scanned += 1;
    for key in root.keys() {
        *summary.root_keys.entry(key.clone()).or_default() += 1;
    }

    let example = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<non-UTF-8 filename>");
    for value in objects {
        visit_value(value, objects, example, summary);
    }
    Ok(())
}

fn visit_value(value: &Value, objects: &[Value], example: &str, summary: &mut SchemaSummary) {
    match value {
        Value::Dictionary(dictionary) => {
            let owner = dictionary_class_name(dictionary, objects);
            if let Some(class_name) = dictionary.get("$classname").and_then(Value::as_string) {
                *summary
                    .class_names
                    .entry(class_name.to_owned())
                    .or_default() += 1;
            }
            for (key, value) in dictionary {
                *summary.all_keys.entry(key.clone()).or_default() += 1;
                if let Some(owner) = owner {
                    summary
                        .key_owners
                        .entry(key.clone())
                        .or_default()
                        .insert(owner.to_owned());
                }
                if is_interesting_key(key) {
                    let resolved = dereference(value, objects).unwrap_or(value);
                    let field = summary.interesting.entry(key.clone()).or_default();
                    field.occurrences += 1;
                    field.files.insert(example.to_owned());
                    if let Some(owner) = owner {
                        field.owners.insert(owner.to_owned());
                    }
                    field.kinds.insert(value_kind(resolved).to_owned());
                    if let Some(scalar) = scalar_description(resolved) {
                        field
                            .value_files
                            .entry(scalar)
                            .or_default()
                            .insert(example.to_owned());
                    }
                }
                visit_inline_value(value, objects, example, summary);
            }
        }
        Value::Array(values) => {
            for value in values {
                visit_value(value, objects, example, summary);
            }
        }
        _ => {}
    }
}

fn visit_inline_value(
    value: &Value,
    objects: &[Value],
    example: &str,
    summary: &mut SchemaSummary,
) {
    if matches!(value, Value::Dictionary(_) | Value::Array(_)) {
        visit_value(value, objects, example, summary);
    }
}

fn dereference<'a>(mut value: &'a Value, objects: &'a [Value]) -> Option<&'a Value> {
    for _ in 0..8 {
        let Value::Uid(uid) = value else {
            return Some(value);
        };
        value = objects.get(uid.get() as usize)?;
    }
    None
}

fn dictionary_class_name<'a>(
    dictionary: &'a plist::Dictionary,
    objects: &'a [Value],
) -> Option<&'a str> {
    dictionary
        .get("$class")
        .and_then(|value| dereference(value, objects))
        .and_then(Value::as_dictionary)
        .and_then(|class| class.get("$classname"))
        .and_then(|value| dereference(value, objects))
        .and_then(Value::as_string)
}

fn is_interesting_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "animation",
        "frame",
        "playback",
        "onion",
        "primarymixed",
        "duration",
        "framerate",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Array(_) => "array",
        Value::Boolean(_) => "bool",
        Value::Data(_) => "data",
        Value::Date(_) => "date",
        Value::Dictionary(_) => "dictionary",
        Value::Integer(_) => "integer",
        Value::Real(_) => "real",
        Value::String(_) => "string",
        Value::Uid(_) => "uid",
        _ => "unknown",
    }
}

fn scalar_description(value: &Value) -> Option<String> {
    match value {
        Value::Boolean(value) => Some(value.to_string()),
        Value::Integer(value) => value
            .as_signed()
            .map(|value| value.to_string())
            .or_else(|| value.as_unsigned().map(|value| value.to_string())),
        Value::Real(value) => Some(value.to_string()),
        Value::String(value) if value.len() <= 64 => Some(value.clone()),
        _ => None,
    }
}

#[derive(Default)]
struct SchemaSummary {
    files_discovered: usize,
    files_scanned: usize,
    files_failed: usize,
    files_skipped: usize,
    root_keys: BTreeMap<String, usize>,
    all_keys: BTreeMap<String, usize>,
    key_owners: BTreeMap<String, BTreeSet<String>>,
    class_names: BTreeMap<String, usize>,
    interesting: BTreeMap<String, FieldSummary>,
}

impl SchemaSummary {
    fn print(&self, show_all_keys: bool) {
        println!("inspection=procreate_archive_schema_v1");
        println!("files_discovered={}", self.files_discovered);
        println!("files_scanned={}", self.files_scanned);
        println!("files_failed={}", self.files_failed);
        println!("files_skipped={}", self.files_skipped);
        for (key, files) in &self.root_keys {
            println!("root_key={key}|files={files}");
        }
        for (key, field) in &self.interesting {
            println!(
                "interesting_key={key}|occurrences={}|files={}|owners={}|kinds={}|distinct_values={}",
                field.occurrences,
                field.files.len(),
                joined(&field.owners),
                joined(&field.kinds),
                field.value_files.len()
            );
            let mut values = field.value_files.iter().collect::<Vec<_>>();
            values.sort_unstable_by(|(left_value, left_files), (right_value, right_files)| {
                right_files
                    .len()
                    .cmp(&left_files.len())
                    .then_with(|| left_value.cmp(right_value))
            });
            for (value, files) in values.into_iter().take(MAX_REPORTED_VALUES) {
                println!(
                    "interesting_value={key}|value={value}|files={}|examples={}|distribution={}",
                    files.len(),
                    examples(files),
                    if field.value_files.len() > MAX_REPORTED_VALUES {
                        "top"
                    } else {
                        "complete"
                    }
                );
            }
        }
        for (class_name, objects) in &self.class_names {
            println!("class={class_name}|objects={objects}");
        }
        if show_all_keys {
            for (key, occurrences) in &self.all_keys {
                println!(
                    "object_key={key}|occurrences={occurrences}|owners={}",
                    self.key_owners.get(key).map(joined).unwrap_or_default()
                );
            }
        }
    }
}

#[derive(Default)]
struct FieldSummary {
    occurrences: usize,
    files: BTreeSet<String>,
    owners: BTreeSet<String>,
    kinds: BTreeSet<String>,
    value_files: BTreeMap<String, BTreeSet<String>>,
}

fn joined(values: &BTreeSet<String>) -> String {
    values.iter().cloned().collect::<Vec<_>>().join(",")
}

fn examples(values: &BTreeSet<String>) -> String {
    values.iter().take(3).cloned().collect::<Vec<_>>().join(",")
}
