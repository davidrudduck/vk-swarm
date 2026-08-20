/// Emission Conformance Guard (Task 021)
///
/// Scans production code for task/execution_process lifecycle writes.
/// Per spec Design "Coverage invariant": every such write must journal an event
/// or be explicitly allowlisted.
#[test]
fn emission_conformance() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut workspace_root = std::path::PathBuf::from(manifest_dir);
    workspace_root.pop(); // db/
    workspace_root.pop(); // crates/
    let crates_dir = workspace_root.join("crates");

    // Scan all .rs files and collect write sites
    let mut inventory = Vec::new();
    walk_crates_recursive(&crates_dir, &crates_dir, &mut inventory);
    inventory.sort();

    // EXPECTED table (fill after first run)
    let expected: &[&str] = &[
        // execution_process/lifecycle.rs
        "db/src/models/execution_process/lifecycle.rs UPDATE execution_processes x6", // :126 INSTRUMENTED (task 007 update_completion); :231/:249/:263/:282/:303 metadata, ALLOWLISTED
        // execution_process/queries.rs
        "db/src/models/execution_process/queries.rs DELETE FROM execution_processes x1", // :533 post-terminal cleanup, ALLOWLISTED
        "db/src/models/execution_process/queries.rs INSERT INTO execution_processes x1", // :473 INSTRUMENTED (task 007)
        "db/src/models/execution_process/queries.rs UPDATE execution_processes x3", // :169 INSTRUMENTED (task 007); :231/:262 metadata, ALLOWLISTED
        // execution_process/sync.rs
        "db/src/models/execution_process/sync.rs UPDATE execution_processes x3", // hive_synced_at metadata, ALLOWLISTED
        // task/archive.rs
        "db/src/models/task/archive.rs UPDATE tasks x4", // archived_at only — outside event vocabulary, ALLOWLISTED
        // task/cleanup.rs
        "db/src/models/task/cleanup.rs DELETE FROM tasks x1", // retention purge of archived terminal tasks, ALLOWLISTED
        // task/hierarchy.rs
        "db/src/models/task/hierarchy.rs UPDATE tasks x2", // :50 INSTRUMENTED (006 update_status); :90 parent_task_id nullify — metadata, ALLOWLISTED
        // task/queries.rs
        "db/src/models/task/queries.rs DELETE FROM tasks x1", // INSTRUMENTED (task 006)
        "db/src/models/task/queries.rs INSERT INTO tasks x1", // INSTRUMENTED (task 006)
        "db/src/models/task/queries.rs UPDATE tasks x1",      // INSTRUMENTED (task 006)
        // task/sync.rs
        "db/src/models/task/sync.rs DELETE FROM tasks x2", // dead/test-only (ADR-0007 soft-unlink), ALLOWLISTED
        "db/src/models/task/sync.rs INSERT INTO tasks x2", // :283 INSTRUMENTED (task 022); :32 sync_from_shared_task dead (zero callers), ALLOWLISTED
        "db/src/models/task/sync.rs UPDATE tasks x13",     // sync metadata only, ALLOWLISTED
        // task_breakdown/queries.rs
        "db/src/models/task_breakdown/queries.rs INSERT INTO tasks x1", // INSTRUMENTED (task 020)
        // server/src/bin/cleanup_duplicate_tasks.rs
        "server/src/bin/cleanup_duplicate_tasks.rs DELETE FROM tasks x1", // one-off ops binary, ALLOWLISTED
    ];

    let actual_set: Vec<&str> = inventory.iter().map(|s| s.as_str()).collect();

    if actual_set != expected {
        eprintln!("ACTUAL INVENTORY:");
        for line in &actual_set {
            eprintln!("  \"{}\",", line);
        }
        eprintln!();

        let expected_set: Vec<&str> = expected.to_vec();
        eprintln!("EXPECTED: {:?}", expected_set);
        eprintln!("ACTUAL: {:?}", actual_set);
        eprintln!();
        eprintln!(
            "New or changed task/execution_process lifecycle write site. Per spec Design 'Coverage invariant' (docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md) every such write must journal a NodeEvent or carry a reviewed allowlist entry here. Instrument it (see Task::update / ExecutionProcess::update_completion for the pattern) or add the entry WITH a written reason — do not silently bump a count."
        );
        panic!("Emission conformance check failed");
    }
}

fn walk_crates_recursive(
    current_dir: &std::path::Path,
    root_crates_dir: &std::path::Path,
    inventory: &mut Vec<String>,
) {
    if !current_dir.exists() {
        return;
    }

    let mut entries = Vec::new();
    if let Ok(dir_entries) = std::fs::read_dir(current_dir) {
        for entry in dir_entries.flatten() {
            entries.push(entry.path());
        }
    }
    entries.sort();

    for path in entries {
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            scan_file(&path, root_crates_dir, inventory);
        } else if path.is_dir() {
            // Skip /target/ and skip anything that is a /tests/ directory
            let path_str = path.to_string_lossy();
            if !path_str.contains("/target/") && !path_str.contains("/tests/") {
                walk_crates_recursive(&path, root_crates_dir, inventory);
            }
        }
    }
}

fn scan_file(
    file_path: &std::path::Path,
    crates_dir: &std::path::Path,
    inventory: &mut Vec<String>,
) {
    // Skip if the file path contains /tests/
    let path_str = file_path.to_string_lossy();
    if path_str.contains("/tests/") {
        return;
    }

    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Strip test region: from `#[cfg(test)]` to EOF if followed by `mod `
    let content = strip_test_region(&content);

    // Strip comments and count patterns
    let mut patterns = Vec::new();
    for line in content.lines() {
        // Strip comments (everything from // onwards)
        let stripped = if let Some(idx) = line.find("//") {
            &line[..idx]
        } else {
            line
        };

        // Count patterns (simple substring match)
        let (insert_tasks, update_tasks, delete_tasks) = count_pattern(
            stripped,
            "INSERT INTO tasks",
            "UPDATE tasks",
            "DELETE FROM tasks",
        );
        let (insert_ep, update_ep, delete_ep) = count_pattern(
            stripped,
            "INSERT INTO execution_processes",
            "UPDATE execution_processes",
            "DELETE FROM execution_processes",
        );

        patterns.push((
            insert_tasks,
            update_tasks,
            delete_tasks,
            insert_ep,
            update_ep,
            delete_ep,
        ));
    }

    // Aggregate counts
    let mut insert_tasks = 0;
    let mut update_tasks = 0;
    let mut delete_tasks = 0;
    let mut insert_ep = 0;
    let mut update_ep = 0;
    let mut delete_ep = 0;

    for (it, ut, dt, iep, uep, dep) in patterns {
        insert_tasks += it;
        update_tasks += ut;
        delete_tasks += dt;
        insert_ep += iep;
        update_ep += uep;
        delete_ep += dep;
    }

    // Build relative path
    let rel_path: String = match file_path.strip_prefix(crates_dir) {
        Ok(p) => p.to_string_lossy().replace("\\", "/"),
        Err(_) => file_path.to_string_lossy().to_string(),
    };
    let rel_path = if let Some(stripped) = rel_path.strip_prefix("../") {
        stripped.to_string()
    } else {
        rel_path
    };

    // Add to inventory
    if insert_tasks > 0 {
        inventory.push(format!("{} INSERT INTO tasks x{}", rel_path, insert_tasks));
    }
    if update_tasks > 0 {
        inventory.push(format!("{} UPDATE tasks x{}", rel_path, update_tasks));
    }
    if delete_tasks > 0 {
        inventory.push(format!("{} DELETE FROM tasks x{}", rel_path, delete_tasks));
    }
    if insert_ep > 0 {
        inventory.push(format!(
            "{} INSERT INTO execution_processes x{}",
            rel_path, insert_ep
        ));
    }
    if update_ep > 0 {
        inventory.push(format!(
            "{} UPDATE execution_processes x{}",
            rel_path, update_ep
        ));
    }
    if delete_ep > 0 {
        inventory.push(format!(
            "{} DELETE FROM execution_processes x{}",
            rel_path, delete_ep
        ));
    }
}

fn strip_test_region(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check if this line is exactly `#[cfg(test)]`
        if trimmed == "#[cfg(test)]" {
            // Look ahead to next non-empty line
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }

            // If next non-empty line starts with `mod `, truncate here
            if j < lines.len() && lines[j].trim().starts_with("mod ") {
                // Include the empty lines but stop before the mod
                result.extend(lines.iter().take(j).skip(i));
                // Stop processing the rest of the file
                break;
            }
        }

        result.push(line);
        i += 1;
    }

    result.join("\n")
}

fn count_pattern(line: &str, p1: &str, p2: &str, p3: &str) -> (usize, usize, usize) {
    let c1 = line.matches(p1).count();
    let c2 = line.matches(p2).count();
    let c3 = line.matches(p3).count();
    (c1, c2, c3)
}
