//! Visual Git conflict detection, marker parsing, and resolution engine.

/// Conflict resolution choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    Current,
    Incoming,
    Both,
}

/// A parsed conflict hunk between HEAD and incoming branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictHunk {
    pub index: usize,
    pub current_branch_name: String,
    pub incoming_branch_name: String,
    pub current_lines: Vec<String>,
    pub incoming_lines: Vec<String>,
    pub choice: Option<ConflictChoice>,
    pub custom_resolution: Option<String>,
}

impl ConflictHunk {
    /// Resolved lines based on selected choice or custom resolution.
    pub fn resolved_lines(&self) -> Vec<String> {
        if let Some(ref custom) = self.custom_resolution {
            return custom.lines().map(|s| s.to_string()).collect();
        }
        match self.choice {
            Some(ConflictChoice::Current) => self.current_lines.clone(),
            Some(ConflictChoice::Incoming) => self.incoming_lines.clone(),
            Some(ConflictChoice::Both) => {
                let mut combined = self.current_lines.clone();
                combined.extend(self.incoming_lines.clone());
                combined
            }
            None => {
                // Return original markers if unresolved
                let mut raw = Vec::new();
                raw.push(format!("<<<<<<< {}", self.current_branch_name));
                raw.extend(self.current_lines.clone());
                raw.push("=======".to_string());
                raw.extend(self.incoming_lines.clone());
                raw.push(format!(">>>>>>> {}", self.incoming_branch_name));
                raw
            }
        }
    }
}

/// Parse conflict markers from a conflicted file.
pub fn parse_conflicts(content: &str) -> (Vec<ConflictHunk>, Vec<String>) {
    let mut hunks = Vec::new();
    let mut non_conflict_lines = Vec::new();

    let mut in_current = false;
    let mut in_incoming = false;
    let mut current_name = "HEAD".to_string();
    let mut current_lines = Vec::new();
    let mut incoming_lines = Vec::new();
    let mut hunk_counter = 0;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_current = true;
            current_name = line.trim_start_matches("<<<<<<<").trim().to_string();
            current_lines.clear();
        } else if line.starts_with("=======") && in_current {
            in_current = false;
            in_incoming = true;
            incoming_lines.clear();
        } else if line.starts_with(">>>>>>>") && in_incoming {
            in_incoming = false;
            let incoming_name = line.trim_start_matches(">>>>>>>").trim().to_string();
            hunks.push(ConflictHunk {
                index: hunk_counter,
                current_branch_name: current_name.clone(),
                incoming_branch_name: incoming_name.clone(),
                current_lines: current_lines.clone(),
                incoming_lines: incoming_lines.clone(),
                choice: None,
                custom_resolution: None,
            });
            hunk_counter += 1;
        } else if in_current {
            current_lines.push(line.to_string());
        } else if in_incoming {
            incoming_lines.push(line.to_string());
        } else {
            non_conflict_lines.push(line.to_string());
        }
    }

    (hunks, non_conflict_lines)
}

/// Apply resolutions to file content.
pub fn apply_resolutions(content: &str, hunks: &[ConflictHunk]) -> String {
    let mut output = Vec::new();
    let mut hunk_idx = 0;
    let mut in_conflict = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            in_conflict = false;
            if let Some(hunk) = hunks.get(hunk_idx) {
                output.extend(hunk.resolved_lines());
            }
            hunk_idx += 1;
        } else if !in_conflict {
            output.push(line.to_string());
        }
    }

    output.join("\n")
}
