use silica::ProcreateFile;
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs::{self, File},
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    let roots = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err("usage: verify_parser_corpus <file-or-directory>...".into());
    }

    let mut paths = Vec::new();
    for root in roots {
        collect_procreate_files(&root, &mut paths)?;
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Err("corpus does not contain any .procreate files".into());
    }

    let mut animation_values = BTreeMap::<(Option<u64>, u64, u64), usize>::new();
    let mut failures = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        eprintln!(
            "checking={}/{} file={}",
            index + 1,
            paths.len(),
            path.display()
        );
        match ProcreateFile::open_reader(File::open(path)?) {
            Ok(document) => {
                if let Some(animation) = document.animation {
                    *animation_values
                        .entry((
                            animation.assist_mode.map(|mode| mode.raw()),
                            animation.playback_mode.raw(),
                            animation.playback_direction.raw(),
                        ))
                        .or_default() += 1;
                }
            }
            Err(error) => failures.push((path, error)),
        }
    }

    println!("verification=parser_corpus_v1");
    println!("documents={}", paths.len());
    println!("parse_failures={}", failures.len());
    for ((assist, mode, direction), count) in animation_values {
        println!(
            "animation_raw=assist:{},mode:{mode},direction:{direction},documents:{count}",
            assist.map_or_else(|| "missing".to_owned(), |value| value.to_string())
        );
    }
    for (path, error) in &failures {
        eprintln!("parse_failure={} error={error}", path.display());
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{} corpus documents failed to parse", failures.len()).into())
    }
}

fn collect_procreate_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    if path.is_file() {
        if has_procreate_extension(path) {
            files.push(path.to_owned());
        }
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_procreate_files(&path, files)?;
        } else if has_procreate_extension(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn has_procreate_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("procreate"))
}
