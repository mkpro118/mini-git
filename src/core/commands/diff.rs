use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;

use crate::core::commands::{
    collect_files_to_process, get_file_contents, resolve_cla_files, FileSource,
};
use crate::core::objects::{self, blob::Blob, tree};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const STAT_WIDTH: usize = 80;
const MAX_THREADS: usize = 8;

#[allow(clippy::struct_excessive_bools)]
struct DiffOpts {
    files: Vec<String>,
    name_only: bool,
    name_status: bool,
    stat: bool,
    diff_filter: Option<String>,
    hunk_context_lines: usize,
    src_prefix: String,
    dst_prefix: String,
    no_prefix: bool,
}

// Data structures for diff computation
#[derive(Debug)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    content: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
enum Change {
    Same,
    Delete,
    Insert,
    Replace,
}

/// List differences
/// This handles the subcommand
///
/// ```bash
/// mini_git diff [options] [ --tree1 TREE1 ] [ --tree2 TREE2 ] [ --files FILE1,FILE2,... ]
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn diff(args: &Namespace) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|_| {
        "Could not determine current working directory".to_owned()
    })?;

    let repo_path = path::repo_find(&cwd)?
        .canonicalize()
        .map_err(|_| "Could not determine repository path".to_owned())?;
    let repo = GitRepository::new(&repo_path)?;

    // Parse arguments
    let name_only = args.get("name-only").is_some();
    let name_status = args.get("name-status").is_some();
    let stat = args.get("stat").is_some();
    let diff_filter = args.get("diff-filter").map(String::as_str);
    let hunk_context_lines = &args["n-context-lines"];
    let src_prefix = &args["src-prefix"];
    let dst_prefix = &args["dst-prefix"];
    let no_prefix = args.get("no-prefix").is_some();

    let Ok(hunk_context_lines) = hunk_context_lines.parse::<usize>() else {
        unreachable!()
    };

    // Resolve the file paths to be relative to the repository root
    let all_files = repo_path.to_str().map_or_else(
        || Err("Failed to determined files to diff".to_owned()),
        |x| Ok(String::from(x)),
    );
    let files = args.get("files").map_or(all_files.as_ref(), Ok)?;
    let resolved_files: Vec<String> = resolve_cla_files(&repo, &cwd, files)?;

    let opts = DiffOpts {
        files: resolved_files,
        name_only,
        name_status,
        stat,
        diff_filter: diff_filter.map(String::from),
        hunk_context_lines,
        src_prefix: src_prefix.to_owned(),
        dst_prefix: dst_prefix.to_owned(),
        no_prefix,
    };

    // Parse tree1 and tree2
    let tree1 = args.get("tree1").filter(|s| *s != "*").map(String::as_str);
    let tree2 = args.get("tree2").filter(|s| *s != "*").map(String::as_str);

    // Finally, switch to the repo root dir to use the resolved paths correctly
    std::env::set_current_dir(&repo_path).map_err(|_| {
        "Could not switch to repository root directory".to_owned()
    })?;

    _diff(repo, tree1, tree2, opts)
}

// Main function simplified to orchestrate the workflow
fn _diff(
    repo: GitRepository,
    tree1: Option<&str>,
    tree2: Option<&str>,
    opts: DiffOpts,
) -> Result<String, String> {
    let (tree1, tree2) = resolve_trees(&repo, tree1, tree2)?;
    let (files1, files2) =
        get_file_contents(&repo, tree1.as_deref(), tree2.as_deref())?;
    let all_files = collect_files_to_process(&files1, &files2, &opts.files);

    process_files_in_parallel(repo, files1, files2, &all_files, opts)
}

// Resolves the tree references based on input parameters
fn resolve_trees<'a>(
    repo: &GitRepository,
    tree1: Option<&'a str>,
    tree2: Option<&'a str>,
) -> Result<(Option<String>, Option<String>), String> {
    match (tree1, tree2) {
        (None, None) => {
            let head = tree::Tree::get_head_tree_sha(repo)?;
            Ok((Some(head), None))
        }
        (Some(tree), None) => {
            let tree_sha = objects::find_object(repo, tree, None, true)?;
            Ok((Some(tree_sha), None))
        }
        (Some(tree1), Some(tree2)) => {
            let tree1_sha = objects::find_object(repo, tree1, None, true)?;
            let tree2_sha = objects::find_object(repo, tree2, None, true)?;
            Ok((Some(tree1_sha), Some(tree2_sha)))
        }
        _ => Err("Invalid tree arguments".to_owned()),
    }
}

// Processes files in parallel using threads
fn process_files_in_parallel(
    repo: GitRepository,
    files1: Vec<FileSource>,
    files2: Vec<FileSource>,
    all_files: &[String],
    opts: DiffOpts,
) -> Result<String, String> {
    let num_threads = usize::min(MAX_THREADS, all_files.len());
    let chunk_size = (all_files.len() + num_threads - 1) / num_threads;

    let file_chunks: Vec<Vec<String>> = all_files
        .chunks(chunk_size)
        .map(<[String]>::to_vec)
        .collect();

    let repo_ref = Arc::new(repo);
    let files1_ref = Arc::new(files1);
    let files2_ref = Arc::new(files2);
    let opts_ref = Arc::new(opts);

    let handles = spawn_worker_threads(
        &repo_ref,
        &file_chunks,
        &files1_ref,
        &files2_ref,
        &opts_ref,
    );
    collect_thread_results(handles)
}

// Spawns worker threads to process file chunks
fn spawn_worker_threads(
    repo: &Arc<GitRepository>,
    file_chunks: &[Vec<String>],
    files1: &Arc<Vec<FileSource>>,
    files2: &Arc<Vec<FileSource>>,
    opts: &Arc<DiffOpts>,
) -> Vec<thread::JoinHandle<Result<Vec<String>, String>>> {
    let mut handles = Vec::new();

    for chunk in file_chunks {
        let repo = repo.clone();
        let files1 = files1.clone();
        let files2 = files2.clone();
        let opts = opts.clone();
        let chunk = chunk.clone();

        let handle = thread::spawn(move || {
            process_file_chunk(&repo, &chunk, &files1, &files2, &opts)
        });

        handles.push(handle);
    }

    handles
}

// Processes a chunk of files in a single thread
fn process_file_chunk(
    repo: &GitRepository,
    chunk: &[String],
    files1: &[FileSource],
    files2: &[FileSource],
    opts: &DiffOpts,
) -> Result<Vec<String>, String> {
    let mut results = Vec::new();

    let tree1_files = files1
        .iter()
        .map(|f| (f.path(), f))
        .collect::<HashMap<_, _>>();
    let tree2_files = files2
        .iter()
        .map(|f| (f.path(), f))
        .collect::<HashMap<_, _>>();

    for file in chunk {
        if let Some(output) =
            process_single_file(repo, file, &tree1_files, &tree2_files, opts)?
        {
            results.push(output);
        }
    }

    Ok(results)
}

// Processes a single file and returns its diff output
fn process_single_file(
    repo: &GitRepository,
    file: &str,
    files1: &HashMap<String, &FileSource>,
    files2: &HashMap<String, &FileSource>,
    opts: &DiffOpts,
) -> Result<Option<String>, String> {
    let content1 = files1.get(file).map(|f| f.contents(repo)).transpose()?;
    let content2 = files2.get(file).map(|f| f.contents(repo)).transpose()?;

    let Some(status) =
        determine_file_status(content1.as_deref(), content2.as_deref())
    else {
        return Ok(None);
    };

    if !should_process_file(status, &opts.diff_filter) {
        return Ok(None);
    }

    Ok(Some(generate_output(
        file,
        status,
        content1.as_deref(),
        content2.as_deref(),
        opts,
    )))
}

// Determines the status of a file (Added, Modified, Deleted)
fn determine_file_status(
    content1: Option<&[u8]>,
    content2: Option<&[u8]>,
) -> Option<char> {
    match (content1, content2) {
        (Some(_), None) => Some('D'),
        (None, Some(_)) => Some('A'),
        (Some(c1), Some(c2)) => {
            if c1 == c2 {
                None
            } else {
                Some('M')
            }
        }
        (None, None) => None,
    }
}

// Checks if a file should be processed based on diff filter
fn should_process_file(status: char, diff_filter: &Option<String>) -> bool {
    if let Some(ref filter) = diff_filter {
        status_matches_filter(status, filter)
    } else {
        true
    }
}

// Generates appropriate output based on options and file status
fn generate_output(
    file: &str,
    status: char,
    content1: Option<&[u8]>,
    content2: Option<&[u8]>,
    opts: &DiffOpts,
) -> String {
    if opts.name_only {
        file.to_string()
    } else if opts.name_status {
        format!("{status}\t{file}")
    } else if opts.stat {
        format_diffstat(file, content1.unwrap_or(&[]), content2.unwrap_or(&[]))
    } else {
        generate_full_diff(file, status, content1, content2, opts)
    }
}

// Generates full diff output based on file status
fn generate_full_diff(
    file: &str,
    status: char,
    content1: Option<&[u8]>,
    content2: Option<&[u8]>,
    opts: &DiffOpts,
) -> String {
    match status {
        'A' => format_addition(
            file,
            content2.unwrap(),
            &opts.src_prefix,
            &opts.dst_prefix,
            opts.no_prefix,
        ),
        'D' => format_deletion(
            file,
            content1.unwrap(),
            &opts.src_prefix,
            &opts.dst_prefix,
            opts.no_prefix,
        ),
        'M' => format_diff(
            file,
            content1.unwrap(),
            content2.unwrap(),
            opts.hunk_context_lines,
            &opts.src_prefix,
            &opts.dst_prefix,
            opts.no_prefix,
        ),
        _ => String::new(),
    }
}

// Collects and sorts results from all threads
fn collect_thread_results(
    handles: Vec<thread::JoinHandle<Result<Vec<String>, String>>>,
) -> Result<String, String> {
    handles
        .into_iter()
        .try_fold(vec![], |mut results, handle| match handle.join() {
            Ok(thread_results) => match thread_results {
                Ok(result) => {
                    results.extend(result);
                    Ok(results)
                }
                Err(msg) => Err(msg),
            },
            Err(_) => Err("A thread panicked during execution".to_string()),
        })
        .map(|mut results| {
            results.sort();
            results.join("\n")
        })
}

fn status_matches_filter(status: char, filter: &str) -> bool {
    // Uppercase letters include, lowercase letters exclude
    let mut include = HashSet::new();
    let mut exclude = HashSet::new();
    for c in filter.chars() {
        if c.is_uppercase() {
            include.insert(c);
        } else if c.is_lowercase() {
            exclude.insert(c.to_ascii_uppercase());
        }
    }
    if !include.is_empty() && !include.contains(&status) {
        return false;
    }
    if exclude.contains(&status) {
        return false;
    }
    true
}

fn compute_diff(old_lines: &[&str], new_lines: &[&str]) -> Vec<Change> {
    let matches = find_matches_optimized(old_lines, new_lines);
    let lcs = build_lcs(&matches);
    generate_changes(old_lines, new_lines, &lcs)
}

fn find_matches_optimized(
    old_lines: &[&str],
    new_lines: &[&str],
) -> Vec<(usize, usize)> {
    let mut new_line_positions: HashMap<&str, Vec<usize>> = HashMap::new();

    // Store all positions for each line
    for (j, &line) in new_lines.iter().enumerate() {
        new_line_positions.entry(line).or_default().push(j);
    }

    let mut matches = Vec::new();
    let mut matched_in_new: HashSet<usize> = HashSet::new();

    for (i, &line) in old_lines.iter().enumerate() {
        if let Some(positions) = new_line_positions.get(line) {
            // Find the first unmatched position
            if let Some(&j) =
                positions.iter().find(|&&j| !matched_in_new.contains(&j))
            {
                matches.push((i, j));
                matched_in_new.insert(j);
            }
        }
    }

    matches.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    matches
}

fn build_lcs(matches: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut paths: Vec<Vec<(usize, usize)>> = Vec::new();

    for &(i, j) in matches {
        let k = match paths
            .binary_search_by(|path| path.last().unwrap().1.cmp(&j))
        {
            Err(k) | Ok(k) => k,
        };

        let mut new_path = if k > 0 {
            paths[k - 1].clone()
        } else {
            Vec::new()
        };
        new_path.push((i, j));

        if k < paths.len() {
            paths[k] = new_path;
        } else {
            paths.push(new_path);
        }
    }

    if let Some(lcs) = paths.last() {
        lcs.clone()
    } else {
        Vec::new()
    }
}

fn generate_changes(
    old_lines: &[&str],
    new_lines: &[&str],
    lcs: &[(usize, usize)],
) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut i = 0;
    let mut j = 0;
    let mut lcs_iter = lcs.iter().peekable();

    while i < old_lines.len() || j < new_lines.len() {
        if let Some(&(lcs_i, lcs_j)) = lcs_iter.peek() {
            if i == *lcs_i && j == *lcs_j {
                // Always compare the actual lines here
                if old_lines[i] == new_lines[j] {
                    changes.push(Change::Same);
                } else {
                    // This case should rarely happen now with improved matching
                    changes.push(Change::Replace);
                }
                i += 1;
                j += 1;
                lcs_iter.next();
            } else if i < *lcs_i && j < *lcs_j {
                // Only mark as Replace if lines are actually different
                if old_lines[i] == new_lines[j] {
                    changes.push(Change::Same);
                } else {
                    changes.push(Change::Replace);
                }
                i += 1;
                j += 1;
            } else if i < *lcs_i {
                // Delete
                changes.push(Change::Delete);
                i += 1;
            } else if j < *lcs_j {
                // Insert
                changes.push(Change::Insert);
                j += 1;
            }
        } else {
            // No more LCS matches
            if i < old_lines.len() && j < new_lines.len() {
                if old_lines[i] == new_lines[j] {
                    changes.push(Change::Same);
                } else {
                    changes.push(Change::Replace);
                }
                i += 1;
                j += 1;
            } else if i < old_lines.len() {
                changes.push(Change::Delete);
                i += 1;
            } else if j < new_lines.len() {
                changes.push(Change::Insert);
                j += 1;
            }
        }
    }

    // Post-process changes to merge adjacent replacements
    let mut optimized_changes = Vec::with_capacity(changes.len());
    let mut i = 0;
    while i < changes.len() {
        if i + 1 < changes.len()
            && matches!(changes[i], Change::Delete)
            && matches!(changes[i + 1], Change::Insert)
        {
            optimized_changes.push(Change::Replace);
            i += 2;
        } else {
            optimized_changes.push(changes[i].clone());
            i += 1;
        }
    }

    optimized_changes
}

#[allow(clippy::too_many_lines)]
fn generate_hunks(
    old_lines: &[&str],
    new_lines: &[&str],
    changes: &[Change],
    hunk_context_lines: usize,
) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current_hunk = String::new();
    let mut old_start = 0;
    let mut new_start = 0;
    let mut old_count = 0;
    let mut new_count = 0;
    let mut last_change_idx = None;

    // Keep track of context lines before changes
    let mut context_buffer = Vec::new();
    // Buffer for storing additions until we can write them after deletions
    let mut additions_buffer = String::new();

    let mut old_line_num = 1;
    let mut new_line_num = 1;

    for (i, change) in changes.iter().enumerate() {
        // Helper function to write context buffer if needed
        let write_context_buffer =
            |current_hunk: &mut String,
             context_buffer: &[(String, usize, usize)],
             old_count: &mut usize,
             new_count: &mut usize| {
                for (line, _, _) in context_buffer {
                    current_hunk.push_str(&format!(" {line}\n"));
                    *old_count += 1;
                    *new_count += 1;
                }
            };

        match change {
            Change::Same => {
                // If we have buffered additions, write them now
                if !additions_buffer.is_empty() {
                    current_hunk.push_str(&additions_buffer);
                    additions_buffer.clear();
                }

                let line = old_lines[old_line_num - 1];

                if last_change_idx.is_none() {
                    // Before any changes, store in context buffer
                    if context_buffer.len() < hunk_context_lines {
                        context_buffer.push((
                            line.to_string(),
                            old_line_num,
                            new_line_num,
                        ));
                    } else {
                        context_buffer.remove(0);
                        context_buffer.push((
                            line.to_string(),
                            old_line_num,
                            new_line_num,
                        ));
                    }
                } else if let Some(last_idx) = last_change_idx {
                    if i - last_idx <= hunk_context_lines {
                        // Within range of last change
                        current_hunk.push_str(&format!(" {line}\n"));
                        old_count += 1;
                        new_count += 1;
                    } else {
                        // End current hunk if exists
                        if !current_hunk.is_empty() {
                            hunks.push(Hunk {
                                old_start,
                                old_count,
                                new_start,
                                new_count,
                                content: current_hunk,
                            });
                            current_hunk = String::new();
                            context_buffer.clear();
                        }
                        context_buffer.push((
                            line.to_string(),
                            old_line_num,
                            new_line_num,
                        ));
                        old_start = old_line_num - context_buffer.len() + 1;
                        new_start = new_line_num - context_buffer.len() + 1;
                        old_count = 0;
                        new_count = 0;
                        last_change_idx = None;
                    }
                }
                old_line_num += 1;
                new_line_num += 1;
            }
            Change::Delete => {
                // Add context buffer if this is the start of a new hunk
                if last_change_idx.is_none() {
                    write_context_buffer(
                        &mut current_hunk,
                        &context_buffer,
                        &mut old_count,
                        &mut new_count,
                    );
                    old_start = old_line_num - context_buffer.len();
                    new_start = new_line_num - context_buffer.len();
                }

                let line = old_lines[old_line_num - 1];
                current_hunk.push_str(&format!("{RED}-{line}{RESET}\n"));
                old_count += 1;
                old_line_num += 1;
                last_change_idx = Some(i);
            }
            Change::Insert => {
                if last_change_idx.is_none() {
                    write_context_buffer(
                        &mut current_hunk,
                        &context_buffer,
                        &mut old_count,
                        &mut new_count,
                    );
                    old_start = old_line_num - context_buffer.len();
                    new_start = new_line_num - context_buffer.len();
                }

                let line = new_lines[new_line_num - 1];
                // Buffer the addition instead of writing it immediately
                additions_buffer.push_str(&format!("{GREEN}+{line}{RESET}\n"));
                new_count += 1;
                new_line_num += 1;
                last_change_idx = Some(i);
            }
            Change::Replace => {
                // Add context buffer if this is the start of a new hunk
                if last_change_idx.is_none() {
                    write_context_buffer(
                        &mut current_hunk,
                        &context_buffer,
                        &mut old_count,
                        &mut new_count,
                    );
                    old_start = old_line_num - context_buffer.len();
                    new_start = new_line_num - context_buffer.len();
                }

                let old_line = old_lines[old_line_num - 1];
                let new_line = new_lines[new_line_num - 1];
                current_hunk.push_str(&format!("{RED}-{old_line}{RESET}\n"));
                additions_buffer
                    .push_str(&format!("{GREEN}+{new_line}{RESET}\n"));
                old_count += 1;
                new_count += 1;
                old_line_num += 1;
                new_line_num += 1;
                last_change_idx = Some(i);
            }
        }

        // If this is a Same change or the last change, write any buffered additions
        if (matches!(change, Change::Same) || i == changes.len() - 1)
            && !additions_buffer.is_empty()
        {
            current_hunk.push_str(&additions_buffer);
            additions_buffer.clear();
        }
    }

    // Add the last hunk if there is one
    if !current_hunk.is_empty() {
        // Make sure to write any remaining buffered additions
        if !additions_buffer.is_empty() {
            current_hunk.push_str(&additions_buffer);
        }
        hunks.push(Hunk {
            old_start,
            old_count,
            new_start,
            new_count,
            content: current_hunk,
        });
    }

    hunks
}

fn format_binary_diff(src_path: &str, dst_path: &str) -> String {
    format!("diff --mini-git {src_path} {dst_path}\nBinary files differ\n")
}

fn format_diff(
    path: &str,
    content1: &[u8],
    content2: &[u8],
    hunk_context_lines: usize,
    src_prefix: &str,
    dst_prefix: &str,
    no_prefix: bool,
) -> String {
    let src_path = if no_prefix {
        path.to_string()
    } else {
        format!("{src_prefix}{path}")
    };
    let dst_path = if no_prefix {
        path.to_string()
    } else {
        format!("{dst_prefix}{path}")
    };

    if Blob::is_binary(content1) || Blob::is_binary(content2) {
        return format_binary_diff(&src_path, &dst_path);
    }

    let old_str = String::from_utf8_lossy(content1);
    let new_str = String::from_utf8_lossy(content2);

    let old_lines: Vec<&str> = old_str.lines().collect();
    let new_lines: Vec<&str> = new_str.lines().collect();

    let changes = compute_diff(&old_lines, &new_lines);
    let hunks =
        generate_hunks(&old_lines, &new_lines, &changes, hunk_context_lines);

    let mut output = String::new();
    output.push_str(&format!(
        "{CYAN}diff --mini-git {src_path} {dst_path}{RESET}\n"
    ));
    output.push_str("index ....\n"); // Simplified index line
    output.push_str(&format!("--- {src_path}\n"));
    output.push_str(&format!("+++ {dst_path}\n"));

    for hunk in hunks {
        output.push_str(&format!(
            "{CYAN}@@ -{},{} +{},{} @@{RESET}\n",
            hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
        ));
        output.push_str(&hunk.content);
    }

    output.push_str(RESET);

    output
}

fn format_binary_addition(src_path: &str, dst_path: &str) -> String {
    format!("diff --mini-git {src_path} {dst_path}\nBinary file added\n")
}

fn format_addition(
    path: &str,
    content: &[u8],
    src_prefix: &str,
    dst_prefix: &str,
    no_prefix: bool,
) -> String {
    let src_path = if no_prefix {
        "/dev/null".to_string()
    } else {
        format!(
            "{}{}",
            src_prefix,
            if src_prefix.ends_with('/') {
                "dev/null"
            } else {
                "/dev/null"
            }
        )
    };
    let dst_path = if no_prefix {
        path.to_string()
    } else {
        format!("{dst_prefix}{path}")
    };

    if Blob::is_binary(content) {
        return format_binary_addition(&src_path, &dst_path);
    }

    let new_str = String::from_utf8_lossy(content);
    let new_lines: Vec<&str> = new_str.lines().collect();

    let mut output = String::new();
    output.push_str(&format!(
        "{CYAN}diff --mini-git {src_path} {dst_path}{RESET}\n"
    ));
    output.push_str("new file mode 100644\n");
    output.push_str(&format!("--- {src_path}\n"));
    output.push_str(&format!("+++ {dst_path}\n"));

    output
        .push_str(&format!("{CYAN}@@ -0,0 +1,{} @@{RESET}\n", new_lines.len()));
    for line in new_lines {
        output.push_str(&format!("{GREEN}+{line}\n"));
    }

    output.push_str(RESET);

    output
}

fn format_binary_deletion(src_path: &str, dst_path: &str) -> String {
    format!("diff --mini-git {src_path} {dst_path}\nBinary file deleted\n")
}

fn format_deletion(
    path: &str,
    content: &[u8],
    src_prefix: &str,
    dst_prefix: &str,
    no_prefix: bool,
) -> String {
    let src_path = if no_prefix {
        path.to_string()
    } else {
        format!("{src_prefix}{path}")
    };
    let dst_path = if no_prefix {
        "/dev/null".to_string()
    } else {
        format!(
            "{}{}",
            dst_prefix,
            if dst_prefix.ends_with('/') {
                "dev/null"
            } else {
                "/dev/null"
            }
        )
    };

    if Blob::is_binary(content) {
        return format_binary_deletion(&src_path, &dst_path);
    }

    let old_str = String::from_utf8_lossy(content);
    let old_lines: Vec<&str> = old_str.lines().collect();

    let mut output = String::new();
    output.push_str(&format!(
        "{CYAN}diff --mini-git {src_path} {dst_path}{RESET}\n"
    ));
    output.push_str("deleted file mode 100644\n");
    output.push_str(&format!("--- {src_path}\n"));
    output.push_str(&format!("+++ {dst_path}\n"));

    output
        .push_str(&format!("{CYAN}@@ -1,{} +0,0 @@{RESET}\n", old_lines.len()));
    for line in old_lines {
        output.push_str(&format!("{RED}-{line}\n"));
    }

    output.push_str(RESET);

    output
}

fn format_diffstat(path: &str, content1: &[u8], content2: &[u8]) -> String {
    // Generate a simple diffstat output
    let old_lines = String::from_utf8_lossy(content1);
    let old_lines: Vec<&str> = old_lines.lines().collect();
    let new_lines = String::from_utf8_lossy(content2);
    let new_lines: Vec<&str> = new_lines.lines().collect();

    let changes = compute_diff(&old_lines, &new_lines);

    let (mut additions, mut deletions) =
        changes.iter().filter(|x| !matches!(x, Change::Same)).fold(
            (0usize, 0usize),
            |(additions, deletions), change| match change {
                Change::Insert => (additions + 1, deletions),
                Change::Delete => (additions, deletions + 1),
                Change::Replace => (additions + 1, deletions + 1),
                Change::Same => unreachable!(),
            },
        );

    // +3 for " | "
    let available_columns = STAT_WIDTH - (path.len() + 3);
    let total_changes = additions + deletions;

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    if total_changes > available_columns {
        let p = (additions as f64 / total_changes as f64)
            * available_columns as f64;
        additions = p as usize;
        deletions = available_columns - additions;
    }

    format!(
        "{path} | {total_changes} {GREEN}{}{RED}{}{RESET}",
        "+".repeat(additions),
        "-".repeat(deletions)
    )
}

/// Make parser for the diff command
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new(
        "Show changes between commits, commit and working tree, etc.",
    );

    parser
        .add_argument("name-only", ArgumentType::Boolean)
        .optional()
        .add_help("Show only names of changed files");

    parser
        .add_argument("name-status", ArgumentType::Boolean)
        .optional()
        .add_help("Show only the name(s) and status of each changed file.");

    parser
        .add_argument("stat", ArgumentType::Boolean)
        .optional()
        .add_help("Generate a diffstat, instead of patch, using 80 columns.");

    parser
        .add_argument("diff-filter", ArgumentType::String)
        .optional()
        .add_help("Select only files that are Added (A), Deleted (D), or Modified (M). Also, these upper-case letters can be downcased to exclude");

    parser
        .add_argument("files", ArgumentType::String)
        .short('f')
        .optional()
        .add_help("Comma-separated list of files to diff");

    parser
        .add_argument("n-context-lines", ArgumentType::Integer)
        .short('l')
        .optional()
        .default("3")
        .add_help("Number of context lines around a diff hunk");

    parser
        .add_argument("src-prefix", ArgumentType::String)
        .optional()
        .default("a/")
        .add_help("Show the given source prefix instead of \"a/\"");

    parser
        .add_argument("dst-prefix", ArgumentType::String)
        .optional()
        .default("b/")
        .add_help("Show the given destination prefix instead of \"b/\"");

    parser
        .add_argument("no-prefix", ArgumentType::Boolean)
        .optional()
        .add_help("Do not show any source or destination prefix");

    parser
        .add_argument("tree1", ArgumentType::String)
        .required()
        .default("*") // * is not a valid branch name
        .add_help("First tree-ish");

    parser
        .add_argument("tree2", ArgumentType::String)
        .required()
        .default("*") // * is not a valid branch name
        .add_help("Second tree-ish");

    parser
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng {
        seed: usize,
        multiplier: usize,
        increment: usize,
    }

    impl Rng {
        #[allow(clippy::cast_possible_truncation)]
        pub fn with(seed: usize, multiplier: usize, increment: usize) -> Self {
            Self {
                seed,
                multiplier,
                increment,
            }
        }

        pub fn gen_range<R>(&mut self, range: R) -> usize
        where
            R: std::ops::RangeBounds<usize>,
        {
            let start = match range.start_bound() {
                std::ops::Bound::Included(x) => *x,
                std::ops::Bound::Excluded(x) => x + 1,
                std::ops::Bound::Unbounded => usize::MIN,
            };
            let end = match range.end_bound() {
                std::ops::Bound::Included(x) => x + 1,
                std::ops::Bound::Excluded(x) => *x,
                std::ops::Bound::Unbounded => usize::MAX,
            };
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };

            self.seed = self
                .seed
                .wrapping_mul(self.multiplier)
                .wrapping_add(self.increment);

            (start.min(end) + self.seed) % start.abs_diff(end)
        }
    }

    // Mock function or struct setups for testing purposes
    fn setup_dummy_files(
    ) -> (HashMap<String, Vec<u8>>, HashMap<String, Vec<u8>>) {
        let mut files1 = HashMap::new();
        let mut files2 = HashMap::new();
        files1.insert("file".to_string(), b"Hello".to_vec());
        files2.insert("file".to_string(), b"Hello World".to_vec());
        (files1, files2)
    }

    #[test]
    fn test_determine_file_status() {
        let (files1, files2) = setup_dummy_files();

        let status_same = determine_file_status(
            files1.get("file").map(|v| &**v),
            files1.get("file").map(|v| &**v),
        );
        assert_eq!(status_same, None);

        let status_insert =
            determine_file_status(None, files2.get("file").map(|v| &**v));
        assert_eq!(status_insert, Some('A'));

        let status_delete =
            determine_file_status(files1.get("file").map(|v| &**v), None);
        assert_eq!(status_delete, Some('D'));

        let status_replace = determine_file_status(
            files1.get("file").map(|v| &**v),
            files2.get("file").map(|v| &**v),
        );
        assert_eq!(status_replace, Some('M'));
    }

    #[test]
    fn test_status_matches_filter() {
        fn generate_permutations(
            letters: &[char],
            current: &mut Vec<char>,
            result: &mut HashSet<String>,
            max_length: usize,
        ) {
            if current.len() == max_length {
                result.insert(current.iter().collect());
                return;
            }

            for &letter in letters {
                current.push(letter);
                let new_letters = letters
                    .iter()
                    .filter(|&c| *c != letter)
                    .copied()
                    .collect::<Vec<_>>();
                generate_permutations(
                    &new_letters,
                    current,
                    result,
                    max_length,
                );
                current.pop();
            }
        }

        let letters = vec!['A', 'D', 'M'];

        let mut combos = HashSet::new();

        // Generate combinations of lengths 0, 1, 2, and 3
        for length in 0..=3 {
            let mut current = vec![];
            generate_permutations(&letters, &mut current, &mut combos, length);
        }

        for letter in "ADMLPI".chars() {
            for combo in &combos {
                assert_eq!(
                    status_matches_filter(letter, combo),
                    combo.contains(letter) || combo.is_empty(),
                    "{letter} '{combo}' | '{}' '{}'",
                    combo.contains(letter),
                    combo.is_empty(),
                );
            }
        }
    }

    #[test]
    fn test_compute_diff_same_content() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 1", "Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert!(changes.iter().all(|change| matches!(change, Change::Same)));
    }

    #[test]
    fn test_compute_diff_with_deletion() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 1", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Delete);
        assert_eq!(changes[2], Change::Same);
    }

    #[test]
    fn test_compute_diff_with_insertion() {
        let old_lines = ["Line 1", "Line 3"];
        let new_lines = ["Line 1", "Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Insert);
        assert_eq!(changes[2], Change::Same);
    }

    #[test]
    fn test_compute_diff_with_replacement() {
        let old_lines = ["Line 1", "Old Line 2", "Line 3"];
        let new_lines = ["Line 1", "New Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        dbg!(&changes);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Replace);
        assert_eq!(changes[2], Change::Same);
    }

    #[test]
    fn test_compute_diff_with_empty_old_lines() {
        let old_lines: [&str; 0] = [];
        let new_lines = ["Line 1", "Line 2"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Insert);
        assert_eq!(changes[1], Change::Insert);
    }

    #[test]
    fn test_compute_diff_with_empty_new_lines() {
        let old_lines = ["Line 1", "Line 2"];
        let new_lines: [&str; 0] = [];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Delete);
        assert_eq!(changes[1], Change::Delete);
    }

    #[test]
    fn test_compute_diff_large_similar_sequences() {
        let old_lines: Vec<_> =
            (0..1000).map(|i| format!("Line {i}")).collect();
        let mut new_lines = old_lines.clone();
        new_lines[500] = "Modified Line".to_string();

        let old_refs: Vec<_> = old_lines.iter().map(String::as_str).collect();
        let new_refs: Vec<_> = new_lines.iter().map(String::as_str).collect();

        let changes = compute_diff(&old_refs, &new_refs);
        assert_eq!(changes.len(), old_lines.len());
        assert_eq!(changes[500], Change::Replace);
    }

    #[test]
    fn test_compute_diff_with_long_common_prefix_suffix() {
        let old_lines = ["A", "B", "C", "D", "E"];
        let new_lines = ["A", "B", "X", "D", "E"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 5);
        assert_eq!(changes[2], Change::Replace);
    }

    #[test]
    fn test_compute_diff_with_multiple_insertions() {
        let old_lines = ["Line 1", "Line 4"];
        let new_lines = ["Line 1", "Line 2", "Line 3", "Line 4"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 4);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Insert);
        assert_eq!(changes[2], Change::Insert);
        assert_eq!(changes[3], Change::Same);
    }

    #[test]
    fn test_compute_diff_with_multiple_deletions() {
        let old_lines = ["Line 1", "Line 2", "Line 3", "Line 4"];
        let new_lines = ["Line 1", "Line 4"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 4);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Delete);
        assert_eq!(changes[2], Change::Delete);
        assert_eq!(changes[3], Change::Same);
    }

    #[test]
    fn test_compute_diff_with_multiple_replacements() {
        let old_lines = ["Line 1", "Line 2", "Line 3", "Line 4"];
        let new_lines = ["Line 1", "New Line 2", "New Line 3", "Line 4"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 4);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Replace);
        assert_eq!(changes[2], Change::Replace);
        assert_eq!(changes[3], Change::Same);
    }

    #[test]
    fn test_compute_diff_completely_different_sequences() {
        let old_lines = ["A", "B", "C"];
        let new_lines = ["X", "Y", "Z"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Replace);
        assert_eq!(changes[1], Change::Replace);
        assert_eq!(changes[2], Change::Replace);
    }

    #[test]
    fn test_compute_diff_reversed_sequences() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 3", "Line 2", "Line 1"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 5);
        assert_eq!(changes[0], Change::Delete);
        assert_eq!(changes[1], Change::Delete);
        assert_eq!(changes[2], Change::Same);
        assert_eq!(changes[3], Change::Insert);
        assert_eq!(changes[4], Change::Insert);
    }

    #[test]
    fn test_compute_diff_interleaved_changes() {
        let old_lines = ["A", "B", "C", "D", "E"];
        let new_lines = ["A", "X", "C", "Y", "E"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 5);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Replace);
        assert_eq!(changes[2], Change::Same);
        assert_eq!(changes[3], Change::Replace);
        assert_eq!(changes[4], Change::Same);
    }

    #[test]
    fn test_compute_diff_leading_trailing_insertions() {
        let old_lines = ["Middle"];
        let new_lines = ["Start", "Middle", "End"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Insert);
        assert_eq!(changes[1], Change::Same);
        assert_eq!(changes[2], Change::Insert);
    }

    #[test]
    fn test_compute_diff_leading_trailing_deletions() {
        let old_lines = ["Start", "Middle", "End"];
        let new_lines = ["Middle"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Delete);
        assert_eq!(changes[1], Change::Same);
        assert_eq!(changes[2], Change::Delete);
    }

    #[test]
    fn test_compute_diff_both_empty_sequences() {
        let old_lines: [&str; 0] = [];
        let new_lines: [&str; 0] = [];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn test_compute_diff_one_empty_one_non_empty() {
        let old_lines: [&str; 0] = [];
        let new_lines = ["Line 1", "Line 2"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Insert);
        assert_eq!(changes[1], Change::Insert);

        let old_lines = ["Line 1", "Line 2"];
        let new_lines: [&str; 0] = [];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Delete);
        assert_eq!(changes[1], Change::Delete);
    }

    #[test]
    fn test_compute_diff_with_repeating_lines() {
        let old_lines = ["Line", "Line", "Line"];
        let new_lines = ["Line", "Line"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Same);
        assert_eq!(changes[2], Change::Delete);
    }

    #[test]
    fn test_compute_diff_with_special_characters() {
        let old_lines = [
            "Line with spaces",
            "Line_with_underscores",
            "Line-with-dashes",
        ];
        let new_lines = [
            "Line with spaces",
            "Line with underscores",
            "Line-with-dashes",
        ];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Replace);
        assert_eq!(changes[2], Change::Same);
    }

    #[test]
    fn test_compute_diff_case_sensitivity() {
        let old_lines = ["Line", "line", "LINE"];
        let new_lines = ["Line", "Line", "Line"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Replace);
        assert_eq!(changes[2], Change::Replace);
    }

    #[test]
    fn test_compute_diff_with_unicode_characters() {
        let old_lines = ["こんにちは", "世界"];
        let new_lines = ["こんにちは", "世界！"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Same);
        assert_eq!(changes[1], Change::Replace);
    }

    #[test]
    fn test_compute_diff_with_whitespace_differences() {
        let old_lines = ["Line with space", "Line with tab\t"];
        let new_lines = ["Line with space ", "Line with tab"];
        let changes = compute_diff(&old_lines, &new_lines);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], Change::Replace);
        assert_eq!(changes[1], Change::Replace);
    }

    #[test]
    fn test_compute_diff_large_random_differences() {
        let size = 10_000;
        let mut rng = Rng::with(0xdead_beef, 0xdead_feed, 0xdead_bea7);

        let old_lines: Vec<_> =
            (0..size).map(|i| format!("Line {i}")).collect();
        let mut new_lines = old_lines.clone();

        // Introduce random replacements
        for _ in 0..(size / 100) {
            let index = rng.gen_range(0..size);
            new_lines[index] = format!("Modified Line {index}");
        }

        // Convert to &str slices
        let old_refs: Vec<&str> =
            old_lines.iter().map(String::as_str).collect();
        let new_refs: Vec<&str> =
            new_lines.iter().map(String::as_str).collect();

        let changes = compute_diff(&old_refs, &new_refs);

        // Check that the changes vector has the correct length
        assert_eq!(changes.len(), size);

        // Count the number of replacements
        let num_replacements =
            changes.iter().filter(|&c| *c == Change::Replace).count();
        assert!(num_replacements > 0);
    }

    #[test]
    fn test_compute_diff_large_insertions() {
        let size = 10_000;

        let old_lines: Vec<_> =
            (0..size).map(|i| format!("Line {i}")).collect();
        let mut new_lines = Vec::with_capacity(size + size / 10);

        // Insert a new line every 10 lines
        for i in 0..size {
            if i % 10 == 0 {
                new_lines.push(format!("Inserted Line {i}"));
            }
            new_lines.push(format!("Line {i}"));
        }

        // Convert to &str slices
        let old_refs: Vec<&str> =
            old_lines.iter().map(String::as_str).collect();
        let new_refs: Vec<&str> =
            new_lines.iter().map(String::as_str).collect();

        let changes = compute_diff(&old_refs, &new_refs);

        // Check that the changes vector has the correct length
        assert_eq!(changes.len(), new_lines.len());

        // Check that the number of insertions is as expected
        let num_insertions =
            changes.iter().filter(|&c| *c == Change::Insert).count();
        assert_eq!(num_insertions, size / 10);
    }

    #[test]
    fn test_compute_diff_large_deletions() {
        let size = 10_000;

        let mut old_lines: Vec<_> = Vec::with_capacity(size + size / 10);
        let new_lines: Vec<_> =
            (0..size).map(|i| format!("Line {i}")).collect();

        // Insert a line every 10 lines in the old sequence
        for i in 0..size {
            if i % 10 == 0 {
                old_lines.push(format!("Deleted Line {i}"));
            }
            old_lines.push(format!("Line {i}"));
        }

        // Convert to &str slices
        let old_refs: Vec<&str> =
            old_lines.iter().map(String::as_str).collect();
        let new_refs: Vec<&str> =
            new_lines.iter().map(String::as_str).collect();

        let changes = compute_diff(&old_refs, &new_refs);

        // Check that the changes vector has the correct length
        assert_eq!(changes.len(), old_lines.len());

        // Check that the number of deletions is as expected
        let num_deletions =
            changes.iter().filter(|&c| *c == Change::Delete).count();
        assert_eq!(num_deletions, size / 10);
    }

    #[test]
    fn test_compute_diff_large_mixed_changes() {
        let size = 10_000;
        let mut rng = Rng::with(0xdead_beef, 0xdead_feed, 0xdead_bea7);

        let old_lines: Vec<_> =
            (0..size).map(|i| format!("Line {i}")).collect();
        let mut new_lines = Vec::with_capacity(size);

        for (i, old_line) in old_lines.iter().enumerate().take(size) {
            let change_type = rng.gen_range(0..3);
            match change_type {
                0 => {
                    // Keep the line the same
                    new_lines.push(old_line.clone());
                }
                1 => {
                    // Modify the line
                    new_lines.push(format!("Modified Line {i}"));
                }
                2 => {
                    // Skip the line (deletion)
                    // Do not push to new_lines
                    continue;
                }
                _ => {}
            }
        }

        // Convert to &str slices
        let old_refs: Vec<&str> =
            old_lines.iter().map(String::as_str).collect();
        let new_refs: Vec<&str> =
            new_lines.iter().map(String::as_str).collect();

        let changes = compute_diff(&old_refs, &new_refs);

        // Check that the changes vector reflects the operations
        assert_eq!(changes.len(), old_refs.len());

        // Verify that there are some of each type of change
        let num_sames = changes.iter().filter(|&c| *c == Change::Same).count();
        let num_replaces =
            changes.iter().filter(|&c| *c == Change::Replace).count();
        let num_deletions =
            changes.iter().filter(|&c| *c == Change::Delete).count();

        assert!(num_sames > 0);
        assert!(num_replaces > 0);
        assert!(num_deletions > 0);
    }

    #[test]
    fn test_compute_diff_large_common_subsequence() {
        let size = 10_000;

        let mut old_lines: Vec<_> = Vec::new();
        let mut new_lines: Vec<_> = Vec::new();

        // Common prefix
        for i in 0..(size / 4) {
            let line = format!("Common Line {i}");
            old_lines.push(line.clone());
            new_lines.push(line);
        }

        // Diverging part
        for i in (size / 4)..(size / 2) {
            old_lines.push(format!("Old Unique Line {i}"));
            new_lines.push(format!("New Unique Line {i}"));
        }

        // Common suffix
        for i in (size / 2)..size {
            let line = format!("Common Line {i}");
            old_lines.push(line.clone());
            new_lines.push(line);
        }

        // Convert to &str slices
        let old_refs: Vec<&str> =
            old_lines.iter().map(String::as_str).collect();
        let new_refs: Vec<&str> =
            new_lines.iter().map(String::as_str).collect();

        let changes = compute_diff(&old_refs, &new_refs);

        // Check that the changes vector has the correct length
        assert_eq!(changes.len(), size);

        // Check that the middle part is correctly identified as replacements
        for change in changes.iter().take(size / 2).skip(size / 4) {
            assert_eq!(*change, Change::Replace);
        }

        // Check that the common prefix and suffix are identified as same
        for change in changes.iter().take(size / 4) {
            assert_eq!(*change, Change::Same);
        }
        for change in changes.iter().take(size).skip(size / 2) {
            assert_eq!(*change, Change::Same);
        }
    }

    #[test]
    fn test_generate_hunks_simple_change() {
        let old_lines = ["Line 1", "Line 2", "Line 3"];
        let new_lines = ["Line 1", "Changed Line 2", "Line 3"];
        let changes = compute_diff(&old_lines, &new_lines);
        let hunks = generate_hunks(&old_lines, &new_lines, &changes, 3);
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 3);
        assert!(hunk.content.contains("-Line 2"));
        assert!(hunk.content.contains("+Changed Line 2"));
    }

    #[test]
    fn test_format_diff_simple_change() {
        let path = "test.txt";
        let content1 = b"Line 1\nLine 2\nLine 3\n";
        let content2 = b"Line 1\nChanged Line 2\nLine 3\n";
        let hunk_context_lines = 3;
        let diff_output = format_diff(
            path,
            content1,
            content2,
            hunk_context_lines,
            "a/",
            "b/",
            false,
        );
        assert!(diff_output.contains("diff --mini-git a/test.txt b/test.txt"));
        assert!(diff_output.contains("--- a/"));
        assert!(diff_output.contains("+++ b/"));
        assert!(diff_output.contains("@@ -1,3 +1,3 @@"));
        assert!(diff_output.contains("-Line 2"));
        assert!(diff_output.contains("+Changed Line 2"));
    }

    #[test]
    fn test_format_binary_diff() {
        let path = "binary_file.bin";
        let output =
            format_binary_diff(&format!("a/{path}"), &format!("b/{path}"));
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary files differ"));
    }

    #[test]
    fn test_format_binary_addition() {
        let path = "binary_file.bin";
        let output =
            format_binary_addition(&format!("a/{path}"), &format!("b/{path}"));
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary file added"));
    }

    #[test]
    fn test_format_binary_deletion() {
        let path = "binary_file.bin";
        let output =
            format_binary_deletion(&format!("a/{path}"), &format!("b/{path}"));
        assert!(output
            .contains("diff --mini-git a/binary_file.bin b/binary_file.bin"));
        assert!(output.contains("Binary file deleted"));
    }

    #[test]
    fn test_format_addition() {
        let path = "new_file.txt";
        let content = b"New content\nLine 2\n";
        let output = format_addition(path, content, "a/", "b/", false);
        assert!(output.contains("diff --mini-git a/dev/null b/new_file.txt"),);
        assert!(output.contains("new file"));
        assert!(output.contains("+++ b/"));
        assert!(output.contains("+New content"));
        assert!(output.contains("+Line 2"));
    }

    #[test]
    fn test_format_deletion() {
        let path = "old_file.txt";
        let content = b"Old content\nLine 2\n";
        let output = format_deletion(path, content, "a/", "b/", false);
        assert!(output.contains("diff --mini-git a/old_file.txt b/dev/null"),);
        assert!(output.contains("deleted file"));
        assert!(output.contains("--- a/"));
        assert!(output.contains("-Old content"));
        assert!(output.contains("-Line 2"));
    }

    #[test]
    fn test_generate_hunks_with_multiple_changes() {
        let old_lines = ["Line 1", "Line 2", "Line 3", "Line 4"];
        let new_lines = ["Line 1", "Changed Line 2", "Line 3", "New Line 4"];
        let changes = compute_diff(&old_lines, &new_lines);
        let hunks = generate_hunks(&old_lines, &new_lines, &changes, 2);
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert!(hunk.content.contains("-Line 2"));
        assert!(hunk.content.contains("+Changed Line 2"));
        assert!(hunk.content.contains("-Line 4"));
        assert!(hunk.content.contains("+New Line 4"));
    }

    #[test]
    fn test_format_diff_with_no_changes() {
        let path = "unchanged.txt";
        let content = b"Line 1\nLine 2\n";
        let diff_output =
            format_diff(path, content, content, 3, "a/", "b/", false);
        // Since there are no changes, diff output should be minimal
        assert!(diff_output
            .contains("diff --mini-git a/unchanged.txt b/unchanged.txt"));
        assert!(diff_output.contains("--- a/"));
        assert!(diff_output.contains("+++ b/"));
        // No hunks should be present
        assert!(!diff_output.contains("@@"));
    }
}
